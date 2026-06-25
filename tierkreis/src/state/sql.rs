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
    sync::{Arc, Mutex},
    time::Duration,
};

use bitvec::vec::BitVec;
use chrono::Utc;
use deadpool::Runtime;
use diesel::SqliteConnection;
use diesel_async::{
    AsyncMigrationHarness,
    pooled_connection::{AsyncDieselConnectionManager, deadpool::Object},
    sync_connection_wrapper::SyncConnectionWrapper,
};
use diesel_migrations::{EmbeddedMigrations, MigrationHarness, embed_migrations};
use miette::{Context, IntoDiagnostic, miette};
use tokio::sync::{RwLock, watch};
use uuid::Uuid;

use crate::{
    event::{Event, NodeStatus, RunningStateUpdate},
    location::Location,
    state::{
        WorkflowState,
        interface::{NodeState, RuntimeState},
        models::{NewNodeOutput, UpsertNodeState},
    },
};
use crate::{
    event::{NodeEvent, WorkflowRunEvent},
    state::{
        interface::RunAttemptUpdated,
        queries::{
            add_run_metadata, insert_default_workflowrun, read_node_state, read_run_metadata,
            read_workflowrun, update_node_state,
        },
    },
};

/// Embedded Diesel migrations
pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!("migrations");

fn run_migrations(conn: Object<SyncConnectionWrapper<SqliteConnection>>) -> miette::Result<()> {
    let mut harness = AsyncMigrationHarness::new(conn);
    harness
        .run_pending_migrations(MIGRATIONS)
        .map_err(|err| miette!("Failed to run SQLite migrations: {err}"))?;

    Ok(())
}

type ConnPool =
    diesel_async::pooled_connection::deadpool::Pool<SyncConnectionWrapper<SqliteConnection>>;

/// Build a connection pool for the given `database_url`.
///
/// # Errors
///
/// Returns an error when the pool cannot be built, a connection cannot be
/// acquired, migrations fail, or `SQLite` pragmas fail to apply.
pub async fn build_conn_pool_with_url(database_url: &str) -> miette::Result<ConnPool> {
    let manager =
        AsyncDieselConnectionManager::<SyncConnectionWrapper<SqliteConnection>>::new(database_url);

    let pool = diesel_async::pooled_connection::deadpool::Pool::builder(manager)
        .runtime(Runtime::Tokio1)
        .wait_timeout(Some(Duration::from_secs(1)))
        .build()
        .into_diagnostic()?;

    let conn = pool
        .get()
        .await
        .into_diagnostic()
        .wrap_err("Error acquiring connection from pool")?;

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
pub async fn build_conn_pool() -> miette::Result<ConnPool> {
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

    let database_url =
        env::var("DATABASE_URL").unwrap_or_else(|_| fallback.to_string_lossy().to_string());

    build_conn_pool_with_url(&database_url).await
}

/// [`SqliteRuntimeState`] implements [`RuntimeState`] but with a `SQLite` backing
/// that will be persisted in a `SQLite` database.
pub struct SqliteRuntimeState {
    pool: ConnPool,
    lock: Arc<RwLock<()>>,
    update_sender: watch::Sender<RunAttemptUpdated>,
    update_receiver: Mutex<Option<watch::Receiver<RunAttemptUpdated>>>,
}

impl SqliteRuntimeState {
    /// Create a new [`SqliteRuntimeState`] instance.
    ///
    /// # Errors
    ///
    /// Will return Err if the `SQLite` database connection pool cannot be established.
    pub async fn try_new() -> miette::Result<Self> {
        let (sender, receiver) = watch::channel(RunAttemptUpdated {
            attempt: 0,
            run_id: Uuid::nil(),
            stopped: false,
        });
        let pool = build_conn_pool()
            .await
            .wrap_err("Failed to establish database connection")?;
        Ok(Self {
            pool,
            lock: Arc::new(RwLock::new(())),
            update_sender: sender,
            update_receiver: Mutex::new(Some(receiver)),
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
        let (sender, receiver) = watch::channel(RunAttemptUpdated {
            attempt: 0,
            run_id: Uuid::nil(),
            stopped: false,
        });
        let pool = build_conn_pool_with_url(&url)
            .await
            .wrap_err("Failed to establish in-memory database")?;
        Ok(Self {
            pool,
            lock: Arc::new(RwLock::new(())),
            update_sender: sender,
            update_receiver: Mutex::new(Some(receiver)),
        })
    }
}

impl Debug for SqliteRuntimeState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "SqliteRuntimeState")
    }
}

impl RuntimeState for SqliteRuntimeState {
    type WorkflowState = SqliteWorkflowState;

    async fn workflow_state(
        &self,
        run_id: Uuid,
        attempt: u32,
    ) -> miette::Result<SqliteWorkflowState> {
        let mut conn = self
            .pool
            .get()
            .await
            .into_diagnostic()
            .wrap_err("Error acquiring connection from pool")?;
        // Insert the default if the value does not yet exist.
        // Cannot return errors from this trait method, so best-effort initialize.
        let run = read_workflowrun(&mut conn, run_id, attempt).await;
        // Should fail?
        if run.is_err() {
            _ = insert_default_workflowrun(&mut conn, run_id, attempt).await?;
        }

        Ok(SqliteWorkflowState {
            pool: self.pool.clone(),
            lock: self.lock.clone(),
            update_sender: self.update_sender.clone(),
            run_id,
            attempt,
        })
    }

    fn listen(&self) -> miette::Result<watch::Receiver<RunAttemptUpdated>> {
        let receiver = {
            let mut receiver = self
                .update_receiver
                .try_lock()
                .map_err(|err| miette!("Failed to listen: {}", err))?;

            receiver.take().ok_or(miette!(
                "Failed to listen: SqlRuntimeState is already being listened to."
            ))?
        };

        Ok(receiver)
    }
}

/// [`SqlWorkflowState`] is an implementation of [`WorkflowState`] that shares storage
/// with [`SqlRuntimeState`].
pub struct SqliteWorkflowState {
    pool: ConnPool,
    // Only one thread can write to an SQLite db at a time, so we use a RWLock to
    // control access to the ConnPool.
    lock: Arc<RwLock<()>>,
    update_sender: watch::Sender<RunAttemptUpdated>,
    run_id: Uuid,
    attempt: u32,
}

impl Debug for SqliteWorkflowState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "SqliteWorkflowState({}, {})", self.run_id, self.attempt)
    }
}

impl WorkflowState for SqliteWorkflowState {
    async fn write(&self, event: Event) -> miette::Result<()> {
        let mut send_workflow_stopped = false;
        let _lock = self.lock.write().await;

        match event {
            Event::WorkflowRun(ref run_event) => {
                Self::handle_run_event(&mut send_workflow_stopped, run_event);
            }
            Event::Node(ref node_event) => self.handle_node_event(node_event).await?,
        }

        self.update_sender.send_modify(|run_attempt_updated| {
            run_attempt_updated.run_id = self.run_id;
            run_attempt_updated.attempt = self.attempt;
            run_attempt_updated.stopped |= send_workflow_stopped;
        });
        Ok(())
    }

    async fn read(&self, location: &Location) -> miette::Result<NodeState> {
        let _lock = self.lock.read().await;
        let mut conn = self
            .pool
            .get()
            .await
            .into_diagnostic()
            .wrap_err("Error acquiring connection from pool")?;
        read_node_state(&mut conn, self.run_id, self.attempt, location).await
    }

    async fn add_metadata(&self, metadata: HashMap<String, String>) -> miette::Result<()> {
        let _lock = self.lock.write().await;
        let mut conn = self
            .pool
            .get()
            .await
            .into_diagnostic()
            .wrap_err("Error acquiring connection from pool")?;
        add_run_metadata(&mut conn, self.run_id, self.attempt, metadata).await
    }

    async fn read_metadata(&self) -> miette::Result<HashMap<String, String>> {
        let _lock = self.lock.read().await;
        let mut conn = self
            .pool
            .get()
            .await
            .into_diagnostic()
            .wrap_err("Error acquiring connection from pool")?;
        read_run_metadata(&mut conn, self.run_id, self.attempt).await
    }
}

impl SqliteWorkflowState {
    fn handle_run_event(send_workflow_stopped: &mut bool, run_event: &WorkflowRunEvent) {
        match run_event {
            WorkflowRunEvent::Started {} => {}
            WorkflowRunEvent::Cancelled {}
            | WorkflowRunEvent::Errored {}
            | WorkflowRunEvent::Completed {} => *send_workflow_stopped = true,
        }
    }

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
                    NodeStatus::Queued => {
                        row.queued_time = Some(now);
                    }
                    NodeStatus::Running { state_update: None } => {
                        row.running_time = Some(now);
                    }
                    NodeStatus::Running {
                        state_update: Some(RunningStateUpdate::Switching { cond }),
                    } => {
                        row.running_time = Some(now);
                        row.cond = Some(cond);
                    }
                    NodeStatus::Running {
                        state_update: Some(RunningStateUpdate::Looping { index }),
                    } => {
                        row.running_time = Some(now);
                        row.loop_index = Some(index.try_into().into_diagnostic()?);
                    }
                    NodeStatus::Running {
                        state_update: Some(RunningStateUpdate::MapStarted { size }),
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
                                        name: port.clone(),
                                        asset_kind: asset_spec.kind.to_string(),
                                        storage_name: asset_spec.storage_name.clone(),
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

        let run_id = Uuid::now_v7();
        let attempt = 0;
        let workflow_state = runtime_state.workflow_state(run_id, attempt).await?;

        let node_state = workflow_state.read(&Location::root()).await?;

        assert_eq!(node_state, NodeState::default());

        Ok(())
    }

    /// Test that we can write and listen for updates.
    #[tokio::test(flavor = "multi_thread")]
    async fn write_and_listen_for_updates() -> miette::Result<()> {
        let runtime_state = SqliteRuntimeState::try_new_in_memory().await?;

        let mut recv = runtime_state.listen()?;

        let run_id = Uuid::now_v7();
        let attempt = 1;
        let workflow_state = runtime_state.workflow_state(run_id, attempt).await?;

        workflow_state
            .write(Event::Node(NodeEvent {
                locs: vec![Location::root()],
                status: NodeStatus::Scheduled,
            }))
            .await?;

        let updated = recv.borrow_and_update();
        assert_eq!(
            *updated,
            RunAttemptUpdated {
                run_id,
                attempt,
                stopped: false,
            }
        );

        Ok(())
    }

    /// Test that we can read and write workflow run state.
    #[tokio::test(flavor = "multi_thread")]
    async fn write_and_read() -> miette::Result<()> {
        let runtime_state = SqliteRuntimeState::try_new_in_memory().await?;

        let run_id = Uuid::now_v7();
        let attempt = 2;
        let workflow_state = runtime_state.workflow_state(run_id, attempt).await?;

        workflow_state
            .write(Event::Node(NodeEvent {
                locs: vec![Location::root()],
                status: NodeStatus::Scheduled,
            }))
            .await?;

        let node_state = workflow_state.read(&Location::root()).await?;

        assert!(node_state.scheduled_time.is_some());

        workflow_state
            .write(Event::Node(NodeEvent {
                locs: vec![Location::root()],
                status: NodeStatus::Queued,
            }))
            .await?;

        let node_state = workflow_state.read(&Location::root()).await?;

        assert!(node_state.scheduled_time.is_some());
        assert!(node_state.queued_time.is_some());

        Ok(())
    }

    /// Test that we can read and write workflow run state.
    #[tokio::test(flavor = "multi_thread")]
    async fn write_and_read_outputs() -> miette::Result<()> {
        let runtime_state = SqliteRuntimeState::try_new_in_memory().await?;

        let run_id = Uuid::now_v7();
        let attempt = 2;
        let workflow_state = runtime_state.workflow_state(run_id, attempt).await?;

        let mut outputs = HashMap::new();
        outputs.insert(
            "foo".to_string(),
            AssetSpec {
                kind: AssetKind::Memory,
                storage_name: "my_cool_storage".to_string(),
                asset_key: AssetKey::new(),
            },
        );
        workflow_state
            .write(Event::Node(NodeEvent {
                locs: vec![Location::root()],
                status: NodeStatus::Complete {
                    outputs: vec![outputs],
                },
            }))
            .await?;

        let node_state = workflow_state.read(&Location::root()).await?;

        assert!(node_state.outputs.is_some());

        Ok(())
    }

    /// Test that we can read and write workflow run state with `map_completed`.
    #[tokio::test(flavor = "multi_thread")]
    async fn write_and_read_map_completed() -> miette::Result<()> {
        let runtime_state = SqliteRuntimeState::try_new_in_memory().await?;

        let run_id = Uuid::now_v7();
        let attempt = 2;
        let workflow_state = runtime_state.workflow_state(run_id, attempt).await?;

        workflow_state
            .write(Event::Node(NodeEvent {
                locs: vec![Location::root()],
                status: NodeStatus::Running {
                    state_update: Some(RunningStateUpdate::MapStarted { size: 2 }),
                },
            }))
            .await?;

        let node_state = workflow_state.read(&Location::root()).await?;

        assert!(node_state.map_completed.is_some());

        let mut bits1 = BitVec::repeat(false, 2);
        bits1.set(0, true);
        workflow_state
            .write(Event::Node(NodeEvent {
                locs: vec![Location::root()],
                status: NodeStatus::Running {
                    state_update: Some(RunningStateUpdate::MapElemComplete {
                        bits: bits1.clone(),
                    }),
                },
            }))
            .await?;

        let node_state = workflow_state.read(&Location::root()).await?;

        assert_eq!(node_state.map_completed, Some(bits1.clone()));

        let mut bits2 = BitVec::repeat(false, 2);
        bits2.set(1, true);
        workflow_state
            .write(Event::Node(NodeEvent {
                locs: vec![Location::root()],
                status: NodeStatus::Running {
                    state_update: Some(RunningStateUpdate::MapElemComplete {
                        bits: bits2.clone(),
                    }),
                },
            }))
            .await?;

        let node_state = workflow_state.read(&Location::root()).await?;

        assert_eq!(node_state.map_completed, Some(bits2.bitor(bits1)));

        Ok(())
    }

    /// Test that we can read and write metadata
    #[tokio::test(flavor = "multi_thread")]
    async fn write_and_read_metadata() -> miette::Result<()> {
        let runtime_state = SqliteRuntimeState::try_new_in_memory().await?;

        let run_id = Uuid::now_v7();
        let attempt = 3;
        let workflow_state = runtime_state.workflow_state(run_id, attempt).await?;

        let metadata = HashMap::from_iter([("foo".to_string(), "bar".to_string())]);
        workflow_state.add_metadata(metadata.clone()).await?;

        let read_metadata = workflow_state.read_metadata().await?;

        assert_eq!(metadata, read_metadata);

        Ok(())
    }
    /// Test that metadata we write gets merged.
    #[tokio::test(flavor = "multi_thread")]
    async fn merge_metadata() -> miette::Result<()> {
        let runtime_state = SqliteRuntimeState::try_new_in_memory().await?;

        let run_id = Uuid::now_v7();
        let attempt = 4;
        let workflow_state = runtime_state.workflow_state(run_id, attempt).await?;

        let mut metadata1 = HashMap::from_iter([("foo".to_string(), "bar".to_string())]);
        workflow_state.add_metadata(metadata1.clone()).await?;

        let metadata2 = HashMap::from_iter([("baz".to_string(), "boo".to_string())]);
        workflow_state.add_metadata(metadata2.clone()).await?;

        let read_metadata = workflow_state.read_metadata().await?;

        metadata1.extend(metadata2.into_iter());
        assert_eq!(metadata1, read_metadata);

        Ok(())
    }
}
