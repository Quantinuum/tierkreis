/*!
This module defines the [`SQliteRuntimeState`] struct which implements [`RuntimeState`]
that can be used by the tierkreis runtime.

These implementations are intended to be used for testing and debugging as their
state is not persisted beyond the lifetime of the process.
*/
use std::{
    collections::HashMap,
    env::{self, home_dir},
    fmt::Debug,
    sync::Arc,
    time::Duration,
};

use bitvec::vec::BitVec;
use chrono::Utc;
use deadpool::{
    Runtime,
    managed::{Hook, HookError, Object},
};
use diesel::{SqliteConnection, sql_query};
use diesel_async::{
    AsyncConnection, AsyncMigrationHarness, RunQueryDsl,
    pooled_connection::AsyncDieselConnectionManager, scoped_futures::ScopedFutureExt,
    sync_connection_wrapper::SyncConnectionWrapper,
};
use diesel_migrations::{EmbeddedMigrations, MigrationHarness, embed_migrations};
use futures::{FutureExt, future::BoxFuture};
use miette::{Context, IntoDiagnostic, miette};
use tokio::sync::watch;
use uuid::Uuid;

use crate::{
    asset_storage::AssetSpec,
    event::{NodeEvent, WorkflowRunEvent},
    graph::WorkflowGraph,
    state::{
        interface::RuntimeWatchState,
        models::{NewWorkflow, NewWorkflowRun, NewWorkflowRunInput, UpsertWorkflowRun},
        queries::{
            WorkflowRunSummary, add_run_attempt_metadata, insert_workflow, insert_workflow_run,
            insert_workflow_run_inputs, list_active_runs, list_workflow_run_summaries,
            read_node_state, read_node_states, read_run_attempt_metadata, read_workflow,
            read_workflow_run, read_workflow_run_inputs, update_node_state, update_workflow_run,
        },
    },
};
use crate::{
    event::{NodeStatus, RunningStateUpdate},
    location::Location,
    state::{
        WorkflowRunState,
        interface::{NodeState, RuntimeState},
        models::{NewNodeOutput, UpsertNodeState},
    },
};

/// Embedded Diesel migrations
pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!("migrations");

fn run_migrations(
    conn: diesel_async::pooled_connection::deadpool::Object<
        SyncConnectionWrapper<SqliteConnection>,
    >,
) -> miette::Result<()> {
    let mut harness = AsyncMigrationHarness::new(conn);
    harness
        .run_pending_migrations(MIGRATIONS)
        .map_err(|err| miette!("Failed to run SQLite migrations: {err}"))?;

    Ok(())
}

/// Type alias for a connection pool to a `SQLite` database.
type ConnPool =
    diesel_async::pooled_connection::deadpool::Pool<SyncConnectionWrapper<SqliteConnection>>;

/// Build a connection pool for the given `database_url`.
///
/// # Errors
///
/// Returns an error when the pool cannot be built, a connection cannot be
/// acquired, migrations fail, or `SQLite` pragmas fail to apply.
pub async fn build_conn_pool_with_url(
    database_url: &str,
    max_size: Option<usize>,
) -> miette::Result<ConnPool> {
    let manager =
        AsyncDieselConnectionManager::<SyncConnectionWrapper<SqliteConnection>>::new(database_url);

    let default_wait = Duration::from_secs(1);

    let mut builder = diesel_async::pooled_connection::deadpool::Pool::builder(manager)
        .runtime(Runtime::Tokio1)
        .wait_timeout(Some(default_wait))
        .post_create(Hook::async_fn(move |mut conn, _metrics| {
            Box::pin(async move {
                let res: diesel::QueryResult<_> = sql_query(format!(
                    "PRAGMA busy_timeout = {}",
                    default_wait.as_millis()
                ))
                .execute(&mut conn)
                .await;
                res.into_diagnostic()
                    .map_err(|err| HookError::message(err.to_string()))?;
                Ok(())
            })
        }));

    if let Some(max_size) = max_size {
        builder = builder.max_size(max_size);
    }

    let pool = builder.build().into_diagnostic()?;

    let mut conn = pool
        .get()
        .await
        .into_diagnostic()
        .wrap_err("Error acquiring connection from pool")?;

    // Note that WAL has no impact when Sqlite is operating in
    // in-memory mode.
    sql_query("PRAGMA journal_mode=WAL")
        .execute(&mut conn)
        .await
        .into_diagnostic()?;

    run_migrations(conn)?;

    Ok(pool)
}

/// Build a connection pool for the `SQLite` database specified by the `DATABASE_URL` environment variable.
///
/// # Errors
///
/// Returns an error when the database directory cannot be created, the pool cannot
/// be built, a connection cannot be acquired, migrations fail, or `SQLite` pragmas
/// fail to apply.
pub async fn build_conn_pool(max_size: Option<usize>) -> miette::Result<ConnPool> {
    let database_url = match env::var("DATABASE_URL") {
        Ok(database_url) => database_url,
        Err(_err) => resolve_default_db_path()
            .wrap_err("No `DATABASE_URL` set, trying default fall back file paths")?
            .to_string_lossy()
            .to_string(),
    };
    build_conn_pool_with_url(&database_url, max_size).await
}

fn resolve_default_db_path() -> Result<std::path::PathBuf, miette::Error> {
    let fallback = home_dir()
        .unwrap_or_else(|| "/tmp".into())
        .join(".tierkreis/checkpoints/tierkreis.sqlite");
    if let Some(parent) = fallback.parent()
        && !parent.exists()
    {
        std::fs::create_dir_all(parent).map_err(|err| {
            miette!(
                "Failed to create directory for SQLite database at {}: {err}",
                fallback.display()
            )
        })?;
    }
    Ok(fallback)
}

/// [`SqliteRuntimeState`] implements [`RuntimeState`] but with a `SQLite` backing
/// that will be persisted in a `SQLite` database.
pub struct SqliteRuntimeState {
    pool: ConnPool,
    update_sender: watch::Sender<RuntimeWatchState>,
    update_receiver: watch::Receiver<RuntimeWatchState>,
}

impl SqliteRuntimeState {
    /// Create a new [`SqliteRuntimeState`] instance.
    ///
    /// # Errors
    ///
    /// Will return Err if the `SQLite` database connection pool cannot be established.
    pub async fn try_new() -> miette::Result<Self> {
        let (sender, receiver) = watch::channel(RuntimeWatchState::default());
        let pool = build_conn_pool(None)
            .await
            .wrap_err("Failed to establish database connection")?;
        let mut conn = pool
            .get()
            .await
            .into_diagnostic()
            .wrap_err("Error acquiring connection from pool")?;
        let interrupted = list_active_runs(&mut conn).await?;
        sender.send_modify(|watch| watch.active_runs.extend(interrupted));
        Ok(Self {
            pool,
            update_sender: sender,
            update_receiver: receiver,
        })
    }

    /// Create a new [`SqliteRuntimeState`] backed by the specified `SQLite` URL.
    ///
    /// # Errors
    ///
    /// Will return Err if the `SQLite` database connection pool cannot be established.
    pub async fn try_new_with_url(database_url: &str) -> miette::Result<Self> {
        let (sender, receiver) = watch::channel(RuntimeWatchState::default());
        let pool = build_conn_pool_with_url(database_url, None)
            .await
            .wrap_err("Failed to establish database connection")?;
        let mut conn = pool
            .get()
            .await
            .into_diagnostic()
            .wrap_err("Error acquiring connection from pool")?;
        let interrupted = list_active_runs(&mut conn).await?;
        sender.send_modify(|watch| watch.active_runs.extend(interrupted));
        Ok(Self {
            pool,
            update_sender: sender,
            update_receiver: receiver,
        })
    }

    /// Create a new [`SqliteRuntimeState`] backed by an isolated in-memory
    /// `SQLite` database. Each call produces a separate database, making this
    /// suitable for parallel tests.
    ///
    /// # Errors
    ///
    /// Will return Err if the in-memory database connection pool cannot be established.
    pub async fn try_new_in_memory() -> miette::Result<Self> {
        let db_name = Uuid::now_v7();
        // `file:name?mode=memory&cache=shared` gives a named in-memory database
        // that all connections in the pool share, avoiding the isolation problem
        // that `:memory:` has with pooled connections.
        let url = format!("file:{db_name}?mode=memory&cache=shared");
        let (sender, receiver) = watch::channel(RuntimeWatchState::default());
        let pool = build_conn_pool_with_url(&url, Some(1))
            .await
            .wrap_err("Failed to establish in-memory database")?;
        let mut conn = pool
            .get()
            .await
            .into_diagnostic()
            .wrap_err("Error acquiring connection from pool")?;
        let interrupted = list_active_runs(&mut conn).await?;
        sender.send_modify(|watch| watch.active_runs.extend(interrupted));
        Ok(Self {
            pool,
            update_sender: sender,
            update_receiver: receiver,
        })
    }

    async fn get_conn(
        &self,
    ) -> miette::Result<Object<AsyncDieselConnectionManager<SyncConnectionWrapper<SqliteConnection>>>>
    {
        let conn = self
            .pool
            .get()
            .await
            .into_diagnostic()
            .wrap_err("Error acquiring connection from pool")?;
        Ok(conn)
    }
}

impl Debug for SqliteRuntimeState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "SqliteRuntimeState")
    }
}

impl RuntimeState for SqliteRuntimeState {
    fn load_workflow(
        &self,
        workflow_id: Uuid,
    ) -> BoxFuture<'_, miette::Result<(Option<String>, WorkflowGraph)>> {
        async move {
            let mut conn = self.get_conn().await?;
            let workflow = read_workflow(&mut conn, workflow_id).await?;
            Ok((
                workflow.name,
                serde_json::from_slice(&workflow.definition).into_diagnostic()?,
            ))
        }
        .boxed()
    }

    fn save_workflow(
        &self,
        name: Option<String>,
        workflow_graph: WorkflowGraph,
    ) -> BoxFuture<'_, miette::Result<Uuid>> {
        async move {
            let id = Uuid::now_v7();
            let workflow = NewWorkflow {
                id: &id.to_string(),
                name: name.as_deref(),
                created_time: Some(Utc::now().naive_utc()),
                definition: &serde_json::to_vec(&workflow_graph).into_diagnostic()?,
            };
            let mut conn = self.get_conn().await?;
            insert_workflow(&mut conn, &workflow)
                .await
                .into_diagnostic()
                .wrap_err("Failed to save workflow")?;
            Ok(id)
        }
        .boxed()
    }

    fn new_workflow_run_state(
        &self,
        workflow_id: Uuid,
        inputs: HashMap<String, crate::asset_storage::AssetSpec>,
    ) -> BoxFuture<'_, miette::Result<Arc<dyn WorkflowRunState>>> {
        async move {
            let mut conn = self.get_conn().await?;
            let run_id = Uuid::now_v7();
            let run_id_str = run_id.to_string();
            conn.transaction(|conn| {
                async {
                    let run = NewWorkflowRun {
                        id: &run_id_str,
                        workflow_id: &workflow_id.to_string(),
                    };
                    insert_workflow_run(conn, &run).await?;

                    let workflow_inputs = inputs.iter().map(|(name, asset)| NewWorkflowRunInput {
                        workflow_run_id: &run_id_str,
                        name,
                        asset_kind: asset.kind.to_string(),
                        storage_name: &asset.storage_name,
                        asset_key: asset.asset_key.to_string(),
                    });

                    insert_workflow_run_inputs(conn, workflow_inputs).await?;
                    Ok::<_, diesel::result::Error>(())
                }
                .scope_boxed()
            })
            .await
            .into_diagnostic()
            .wrap_err("Failed to insert new workflow run")?;

            self.update_sender.send_modify(|active_runs| {
                active_runs.active_runs.insert((run_id, 0));
            });

            let state = SqliteWorkflowRunState {
                pool: self.pool.clone(),
                update_sender: self.update_sender.clone(),
                workflow_id,
                run_id,
                attempt: 0,
            };
            let state: Arc<dyn WorkflowRunState> = Arc::new(state);
            Ok(state)
        }
        .boxed()
    }

    fn load_workflow_run_state(
        &self,
        run_id: Uuid,
        attempt: u32,
    ) -> BoxFuture<'_, miette::Result<Arc<dyn WorkflowRunState>>> {
        async move {
            let mut conn = self.get_conn().await?;
            let run = read_workflow_run(&mut conn, run_id, attempt).await?;
            let workflow_id = run.0.workflow_id.parse().into_diagnostic()?;

            let state = SqliteWorkflowRunState {
                pool: self.pool.clone(),
                update_sender: self.update_sender.clone(),
                workflow_id,
                run_id,
                attempt,
            };
            let state: Arc<dyn WorkflowRunState> = Arc::new(state);
            Ok(state)
        }
        .boxed()
    }

    fn listen(&self) -> watch::Receiver<RuntimeWatchState> {
        self.update_receiver.clone()
    }

    fn list_workflow_run_summaries(
        &self,
    ) -> BoxFuture<'_, miette::Result<Vec<WorkflowRunSummary>>> {
        async move {
            let mut conn = self.get_conn().await?;
            list_workflow_run_summaries(&mut conn).await
        }
        .boxed()
    }
}

/// [`SqlWorkflowRunState`] is an implementation of [`WorkflowRunState`] that shares storage
/// with [`SqlRuntimeState`].
pub struct SqliteWorkflowRunState {
    pool: ConnPool,
    update_sender: watch::Sender<RuntimeWatchState>,
    workflow_id: Uuid,
    run_id: Uuid,
    attempt: u32,
}

impl Debug for SqliteWorkflowRunState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "SqliteWorkflowRunState({}, {})",
            self.run_id, self.attempt
        )
    }
}

impl WorkflowRunState for SqliteWorkflowRunState {
    fn workflow_id(&self) -> Uuid {
        self.workflow_id
    }

    fn run_id(&self) -> Uuid {
        self.run_id
    }

    fn attempt(&self) -> u32 {
        self.attempt
    }

    fn load_inputs(&self) -> BoxFuture<'_, miette::Result<HashMap<String, AssetSpec>>> {
        async move {
            let mut conn = self.get_conn().await?;
            read_workflow_run_inputs(&mut conn, &self.run_id.to_string()).await
        }
        .boxed()
    }

    fn write(&self, event: WorkflowRunEvent) -> BoxFuture<'_, miette::Result<()>> {
        async move {
            let mut send_workflow_stopped = false;
            let now = Utc::now().naive_utc();
            let mut workflow_update = UpsertWorkflowRun::default();

            match event {
                WorkflowRunEvent::Started {} => {
                    workflow_update.started_time = Some(now);
                }
                WorkflowRunEvent::Queued {} => workflow_update.queued_time = Some(now),
                WorkflowRunEvent::Cancelled {} => {
                    workflow_update.cancelled_time = Some(now);
                    send_workflow_stopped = true;
                }
                WorkflowRunEvent::Errored {} => {
                    workflow_update.error_time = Some(now);
                    send_workflow_stopped = true;
                }
                WorkflowRunEvent::Completed {} => {
                    workflow_update.complete_time = Some(now);
                    send_workflow_stopped = true;
                }
                WorkflowRunEvent::NodeEvent(ref node_event) => {
                    // TODO: can we receive node events before start is set?
                    self.handle_node_event(node_event).await?;
                }
            }

            if !matches!(event, WorkflowRunEvent::NodeEvent(_)) {
                let attempt = self.attempt.try_into().into_diagnostic()?;
                let mut conn = self.get_conn().await?;
                update_workflow_run(
                    &mut conn,
                    &self.run_id.to_string(),
                    attempt,
                    workflow_update,
                )
                .await?;
            }

            self.update_sender.send_modify(|run_attempt_updated| {
                if send_workflow_stopped {
                    run_attempt_updated
                        .active_runs
                        .remove(&(self.run_id, self.attempt));
                } else {
                    run_attempt_updated
                        .active_runs
                        .insert((self.run_id, self.attempt));
                }
            });
            Ok(())
        }
        .boxed()
    }

    fn read<'a>(&'a self, location: &'a Location) -> BoxFuture<'a, miette::Result<NodeState>> {
        async move {
            let mut conn = self.get_conn().await?;
            read_node_state(&mut conn, self.run_id, self.attempt, location).await
        }
        .boxed()
    }

    fn read_many<'a>(
        &'a self,
        locations: &'a mut (dyn Iterator<Item = Location> + Send),
    ) -> BoxFuture<'a, miette::Result<HashMap<Location, NodeState>>> {
        async move {
            let mut conn = self.get_conn().await?;
            read_node_states(&mut conn, self.run_id, self.attempt, locations).await
        }
        .boxed()
    }

    fn add_metadata(&self, metadata: HashMap<String, String>) -> BoxFuture<'_, miette::Result<()>> {
        async move {
            let mut conn = self.get_conn().await?;
            add_run_attempt_metadata(&mut conn, self.run_id, self.attempt, metadata).await
        }
        .boxed()
    }

    fn read_metadata(&self) -> BoxFuture<'_, miette::Result<HashMap<String, String>>> {
        async move {
            let mut conn = self.get_conn().await?;
            read_run_attempt_metadata(&mut conn, self.run_id, self.attempt).await
        }
        .boxed()
    }
}

impl SqliteWorkflowRunState {
    async fn get_conn(
        &self,
    ) -> miette::Result<Object<AsyncDieselConnectionManager<SyncConnectionWrapper<SqliteConnection>>>>
    {
        let conn = self
            .pool
            .get()
            .await
            .into_diagnostic()
            .wrap_err("Error acquiring connection from pool")?;
        Ok(conn)
    }

    #[allow(clippy::too_many_lines)]
    async fn handle_node_event(&self, event: &NodeEvent) -> miette::Result<()> {
        let attempt = self.attempt.try_into().into_diagnostic()?;
        let now = Utc::now().naive_utc();
        let mut node_outputs = Vec::new();
        let node_updates = event
            .locs
            .iter()
            .enumerate()
            .map(|(index, loc)| {
                let mut row = UpsertNodeState {
                    run_id: self.run_id.to_string(),
                    attempt,
                    node_location: loc.clone(),
                    ..Default::default()
                };
                match event.status {
                    NodeStatus::Scheduled => {
                        row.scheduled_time = Some(now);
                    }
                    NodeStatus::Queued { ref handle } => {
                        row.queued_time = Some(now);
                        row.handle.clone_from(handle);
                    }
                    NodeStatus::Running {
                        state_update: None, ..
                    } => {
                        row.running_time = Some(now);
                    }
                    NodeStatus::Running {
                        state_update: Some(RunningStateUpdate::Switching { cond }),
                        ..
                    } => {
                        row.running_time = Some(now);
                        row.cond = Some(cond);
                    }
                    NodeStatus::Running {
                        state_update: Some(RunningStateUpdate::Looping { index }),
                        ..
                    } => {
                        row.running_time = Some(now);
                        row.loop_index = Some(index.try_into().into_diagnostic()?);
                    }
                    NodeStatus::Running {
                        state_update: Some(RunningStateUpdate::MapStarted { size }),
                        ..
                    } => {
                        row.running_time = Some(now);
                        row.map_size = Some(size.try_into().into_diagnostic()?);
                        row.map_completed = Some(
                            BitVec::<u8>::repeat(false, size.try_into().into_diagnostic()?)
                                .into_vec(),
                        );
                    }
                    NodeStatus::Running {
                        state_update: Some(RunningStateUpdate::MapElemComplete { ref bits }),
                        ..
                    } => {
                        row.running_time = Some(now);
                        row.map_completed = Some(bits.clone().into_vec());
                    }
                    NodeStatus::Complete { ref outputs } => {
                        row.complete_time = Some(now);
                        if let Some(output) = outputs.get(index) {
                            for (port, asset_spec) in output {
                                node_outputs.push((
                                    loc.clone(),
                                    NewNodeOutput {
                                        name: port,
                                        asset_kind: asset_spec.kind.to_string(),
                                        storage_name: &asset_spec.storage_name,
                                        asset_key: asset_spec.asset_key.to_string(),
                                    },
                                ));
                            }
                        }
                    }
                    NodeStatus::Cancelled => {
                        row.cancelled_time = Some(now);
                    }
                    NodeStatus::Error {
                        ref error,
                        ref detail,
                    } => {
                        row.error_time = Some(now);
                        row.error = Some(error.clone());
                        row.error_detail.clone_from(detail);
                    }
                }

                Ok(row)
            })
            .collect::<miette::Result<Vec<_>>>()?;

        let mut conn = self
            .pool
            .get()
            .await
            .into_diagnostic()
            .wrap_err("Error acquiring connection from pool")?;

        update_node_state(
            &mut conn,
            &self.run_id.to_string(),
            attempt,
            node_updates,
            node_outputs,
        )
        .await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;
    use std::ops::BitOr;

    use crate::{
        asset_storage::{AssetKey, AssetKind, AssetSpec},
        event::NodeStatus,
    };

    use super::*;

    /// Test that reading a location returns the default value.
    #[tokio::test(flavor = "multi_thread")]
    async fn read_location_returns_default() -> miette::Result<()> {
        let runtime_state = SqliteRuntimeState::try_new_in_memory().await?;

        let workflow_id = runtime_state
            .save_workflow(None, WorkflowGraph::new([]))
            .await?;
        let workflow_run_state = runtime_state
            .new_workflow_run_state(workflow_id, HashMap::new())
            .await?;

        let node_state = workflow_run_state.read(&Location::root()).await?;

        assert_eq!(node_state, NodeState::default());

        Ok(())
    }

    /// Test that we can write and listen for updates.
    #[tokio::test(flavor = "multi_thread")]
    async fn write_and_listen_for_updates() -> miette::Result<()> {
        let runtime_state = SqliteRuntimeState::try_new_in_memory().await?;

        let mut recv = runtime_state.listen();

        let workflow_id = runtime_state
            .save_workflow(None, WorkflowGraph::new([]))
            .await?;
        let workflow_run_state = runtime_state
            .new_workflow_run_state(workflow_id, HashMap::new())
            .await?;

        workflow_run_state
            .write(WorkflowRunEvent::NodeEvent(NodeEvent {
                locs: vec![Location::root()],
                status: NodeStatus::Scheduled,
            }))
            .await?;

        let updated = recv.borrow_and_update();
        let run_id = workflow_run_state.run_id();
        let attempt = workflow_run_state.attempt();
        let expected_runs = HashSet::from_iter([(run_id, attempt)]);
        assert_eq!(
            *updated,
            RuntimeWatchState {
                active_runs: expected_runs,
            }
        );

        Ok(())
    }

    /// Test that we can read and write workflow run state.
    #[tokio::test(flavor = "multi_thread")]
    async fn write_and_read() -> miette::Result<()> {
        let runtime_state = SqliteRuntimeState::try_new_in_memory().await?;

        let workflow_id = runtime_state
            .save_workflow(None, WorkflowGraph::new([]))
            .await?;
        let workflow_run_state = runtime_state
            .new_workflow_run_state(workflow_id, HashMap::new())
            .await?;

        workflow_run_state
            .write(WorkflowRunEvent::NodeEvent(NodeEvent {
                locs: vec![Location::root()],
                status: NodeStatus::Scheduled,
            }))
            .await?;

        let node_state = workflow_run_state.read(&Location::root()).await?;

        assert!(node_state.scheduled_time.is_some());

        workflow_run_state
            .write(WorkflowRunEvent::NodeEvent(NodeEvent {
                locs: vec![Location::root()],
                status: NodeStatus::Queued { handle: None },
            }))
            .await?;

        let node_state = workflow_run_state.read(&Location::root()).await?;

        assert!(node_state.scheduled_time.is_some());
        assert!(node_state.queued_time.is_some());

        Ok(())
    }

    /// Test that we can read and write workflow run state.
    #[tokio::test(flavor = "multi_thread")]
    async fn write_and_read_outputs() -> miette::Result<()> {
        let runtime_state = SqliteRuntimeState::try_new_in_memory().await?;

        let workflow_id = runtime_state
            .save_workflow(None, WorkflowGraph::new([]))
            .await?;
        let workflow_run_state = runtime_state
            .new_workflow_run_state(workflow_id, HashMap::new())
            .await?;

        let mut outputs = HashMap::new();
        outputs.insert(
            "foo".to_string(),
            AssetSpec {
                kind: AssetKind::Memory,
                storage_name: "my_cool_storage".to_string(),
                asset_key: AssetKey::new(),
            },
        );
        workflow_run_state
            .write(WorkflowRunEvent::NodeEvent(NodeEvent {
                locs: vec![Location::root()],
                status: NodeStatus::Complete {
                    outputs: vec![outputs],
                },
            }))
            .await?;

        let node_state = workflow_run_state.read(&Location::root()).await?;

        assert!(node_state.outputs.is_some());

        Ok(())
    }

    /// Test that we can read and write workflow run state with `map_completed`.
    #[tokio::test(flavor = "multi_thread")]
    async fn write_and_read_map_completed() -> miette::Result<()> {
        let runtime_state = SqliteRuntimeState::try_new_in_memory().await?;

        let workflow_id = runtime_state
            .save_workflow(None, WorkflowGraph::new([]))
            .await?;
        let workflow_run_state = runtime_state
            .new_workflow_run_state(workflow_id, HashMap::new())
            .await?;

        workflow_run_state
            .write(WorkflowRunEvent::NodeEvent(NodeEvent {
                locs: vec![Location::root()],
                status: NodeStatus::Running {
                    state_update: Some(RunningStateUpdate::MapStarted { size: 2 }),
                },
            }))
            .await?;

        let node_state = workflow_run_state.read(&Location::root()).await?;

        assert!(node_state.map_completed.is_some());

        let mut bits1 = BitVec::repeat(false, 2);
        bits1.set(0, true);
        workflow_run_state
            .write(WorkflowRunEvent::NodeEvent(NodeEvent {
                locs: vec![Location::root()],
                status: NodeStatus::Running {
                    state_update: Some(RunningStateUpdate::MapElemComplete {
                        bits: bits1.clone(),
                    }),
                },
            }))
            .await?;

        let node_state = workflow_run_state.read(&Location::root()).await?;

        assert_eq!(node_state.map_completed, Some(bits1.clone()));

        let mut bits2 = BitVec::repeat(false, 2);
        bits2.set(1, true);
        workflow_run_state
            .write(WorkflowRunEvent::NodeEvent(NodeEvent {
                locs: vec![Location::root()],
                status: NodeStatus::Running {
                    state_update: Some(RunningStateUpdate::MapElemComplete {
                        bits: bits2.clone(),
                    }),
                },
            }))
            .await?;

        let node_state = workflow_run_state.read(&Location::root()).await?;

        assert_eq!(node_state.map_completed, Some(bits2.bitor(bits1)));

        Ok(())
    }

    /// Test that we can read and write metadata
    #[tokio::test(flavor = "multi_thread")]
    async fn write_and_read_metadata() -> miette::Result<()> {
        let runtime_state = SqliteRuntimeState::try_new_in_memory().await?;

        let workflow_id = runtime_state
            .save_workflow(None, WorkflowGraph::new([]))
            .await?;
        let workflow_run_state = runtime_state
            .new_workflow_run_state(workflow_id, HashMap::new())
            .await?;

        let metadata = HashMap::from_iter([("foo".to_string(), "bar".to_string())]);
        workflow_run_state.add_metadata(metadata.clone()).await?;

        let read_metadata = workflow_run_state.read_metadata().await?;

        assert_eq!(metadata, read_metadata);

        Ok(())
    }
    /// Test that metadata we write gets merged.
    #[tokio::test(flavor = "multi_thread")]
    async fn merge_metadata() -> miette::Result<()> {
        let runtime_state = SqliteRuntimeState::try_new_in_memory().await?;

        let workflow_id = runtime_state
            .save_workflow(None, WorkflowGraph::new([]))
            .await?;
        let workflow_run_state = runtime_state
            .new_workflow_run_state(workflow_id, HashMap::new())
            .await?;

        let mut metadata1 = HashMap::from_iter([("foo".to_string(), "bar".to_string())]);
        workflow_run_state.add_metadata(metadata1.clone()).await?;

        let metadata2 = HashMap::from_iter([("baz".to_string(), "boo".to_string())]);
        workflow_run_state.add_metadata(metadata2.clone()).await?;

        let read_metadata = workflow_run_state.read_metadata().await?;

        metadata1.extend(metadata2.into_iter());
        assert_eq!(metadata1, read_metadata);

        Ok(())
    }
}
