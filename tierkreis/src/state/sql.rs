/*!
This module defines the [`SQliteRuntimeState`] struct which implements [`RuntimeState`]
that can be used by the tierkreis runtime.

These implementations are intended to be used for testing and debugging as their
state is not persisted beyond the lifetime of the process.
*/
use std::{
    collections::HashMap,
    env::{self, home_dir},
    sync::{Arc, Mutex},
};

use bitvec::vec::BitVec;
use chrono::Utc;
use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool};
use diesel_migrations::{EmbeddedMigrations, MigrationHarness, embed_migrations};
use futures::{
    FutureExt, SinkExt, StreamExt,
    channel::mpsc,
    future::{self, BoxFuture},
    stream::BoxStream,
};
use miette::{IntoDiagnostic, miette};
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

fn run_migrations(connection: &mut SqliteConnection) -> miette::Result<()> {
    connection
        .run_pending_migrations(MIGRATIONS)
        .map_err(|err| miette!("Failed to run SQLite migrations: {err}"))?;

    Ok(())
}

/// Build a connection pool for the given `database_url`.
///
/// # Errors
///
/// Returns an error when the pool cannot be built, a connection cannot be
/// acquired, migrations fail, or `SQLite` pragmas fail to apply.
pub fn establish_connection_with_url(
    database_url: &str,
) -> miette::Result<Pool<ConnectionManager<SqliteConnection>>> {
    let manager = ConnectionManager::<SqliteConnection>::new(database_url);

    let pool = Pool::builder()
        .build(manager)
        .map_err(|_| miette!("Error connecting to {}", database_url))?;

    let mut conn = pool
        .get()
        .map_err(|err| miette!("Error acquiring connection for SQLite setup: {err}"))?;
    run_migrations(&mut conn)?;

    diesel::sql_query("PRAGMA busy_timeout = 5000;")
        .execute(&mut conn)
        .map_err(|err| miette!("Failed to apply SQLite busy_timeout: {err}"))?;
    diesel::sql_query("PRAGMA journal_mode = WAL;")
        .execute(&mut conn)
        .map_err(|err| miette!("Failed to apply SQLite WAL mode: {err}"))?;

    Ok(pool)
}

/// Build a connection pool for the `SQLite` database specified by the `DATABASE_URL` environment variable.
///
/// # Errors
///
/// Returns an error when the database directory cannot be created, the pool cannot
/// be built, a connection cannot be acquired, migrations fail, or `SQLite` pragmas
/// fail to apply.
pub fn establish_connection() -> miette::Result<Pool<ConnectionManager<SqliteConnection>>> {
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

    establish_connection_with_url(&database_url)
}

/// [`SqliteRuntimeState`] implements [`RuntimeState`] but with a `SQLite` backing
/// that will be persisted in a `SQLite` database.
#[derive(Debug)]
pub struct SqliteRuntimeState {
    connection: Arc<Pool<ConnectionManager<SqliteConnection>>>,
    //inner: Arc<InMemoryRuntimeStateInner>,
    update_sender: mpsc::Sender<RunAttemptUpdated>,
    update_receiver: Mutex<Option<mpsc::Receiver<RunAttemptUpdated>>>,
}

impl SqliteRuntimeState {
    /// Create a new [`SqliteRuntimeState`] instance.
    ///
    /// # Panics
    ///
    /// Panics if the `SQLite` database connection pool cannot be established.
    #[must_use]
    pub fn new() -> Self {
        // TODO: This channel clogs up easily if left un-checked.
        let (sender, receiver) = mpsc::channel(1024);
        let connection = establish_connection().expect("Failed to establish database connection");
        Self {
            connection: Arc::new(connection),
            update_sender: sender,
            update_receiver: Mutex::new(Some(receiver)),
        }
    }

    /// Create a new [`SqliteRuntimeState`] backed by an isolated in-memory
    /// `SQLite` database. Each call produces a separate database, making this
    /// suitable for parallel tests.
    ///
    /// # Panics
    ///
    /// Panics if the in-memory database connection pool cannot be established.
    #[cfg(test)]
    #[must_use]
    pub fn new_in_memory() -> Self {
        let db_name = Uuid::now_v7();
        // `file:name?mode=memory&cache=shared` gives a named in-memory database
        // that all connections in the pool share, avoiding the isolation problem
        // that `:memory:` has with pooled connections.
        let url = format!("file:{db_name}?mode=memory&cache=shared");
        let (sender, receiver) = mpsc::channel(1024);
        let connection =
            establish_connection_with_url(&url).expect("Failed to establish in-memory database");
        Self {
            connection: Arc::new(connection),
            update_sender: sender,
            update_receiver: Mutex::new(Some(receiver)),
        }
    }
}

impl Default for SqliteRuntimeState {
    fn default() -> Self {
        Self::new()
    }
}

impl RuntimeState for SqliteRuntimeState {
    fn workflow_state(&self, run_id: Uuid, attempt: u32) -> Arc<dyn WorkflowState> {
        // Insert the default if the value does not yet exist.
        // Cannot return errors from this trait method, so best-effort initialize.
        let run = read_workflowrun(run_id, attempt, &self.connection);
        // Should fail?
        if run.is_err() {
            _ = insert_default_workflowrun(run_id, attempt, &self.connection);
        }

        Arc::new(SqliteWorkflowState {
            global_state: self.connection.clone(),
            update_sender: self.update_sender.clone(),
            run_id,
            attempt,
        })
    }

    fn listen(&self) -> miette::Result<BoxStream<'static, RunAttemptUpdated>> {
        let receiver = {
            let mut receiver = self
                .update_receiver
                .try_lock()
                .map_err(|err| miette!("Failed to listen: {}", err))?;

            receiver.take().ok_or(miette!(
                "Failed to listen: SqlRuntimeState is already being listened to."
            ))?
        };

        Ok(receiver.boxed())
    }
}

/// [`SqlWorkflowState`] is an implementation of [`WorkflowState`] that shares storage
/// with [`SqlRuntimeState`].
#[derive(Debug)]
pub struct SqliteWorkflowState {
    global_state: Arc<Pool<ConnectionManager<SqliteConnection>>>,
    update_sender: mpsc::Sender<RunAttemptUpdated>,
    run_id: Uuid,
    attempt: u32,
}

impl SqliteWorkflowState {}

impl WorkflowState for SqliteWorkflowState {
    fn write(&self, event: Event) -> BoxFuture<'_, miette::Result<()>> {
        let mut send_update = false;
        let mut send_workflow_stopped = false;

        let res = match event {
            Event::WorkflowRun(run_event) => {
                let _: () = Self::handle_run_event(&mut send_workflow_stopped, &run_event);
                Ok(())
            }
            Event::Node(node_event) => self.handle_node_event(node_event, &mut send_update),
        };

        if res.is_err() {
            return future::ready(res).boxed();
        }

        let mut update_sender = self.update_sender.clone();
        async move {
            if send_update || send_workflow_stopped {
                update_sender
                    .send(RunAttemptUpdated {
                        run_id: self.run_id,
                        attempt: self.attempt,
                        stopped: send_workflow_stopped,
                    })
                    .await
                    .map_err(|err| miette!("Send failed: {err}"))?;
            }
            Ok(())
        }
        .boxed()
    }

    fn read(&self, location: &Location) -> BoxFuture<'_, miette::Result<NodeState>> {
        let node_state = read_node_state(self.run_id, self.attempt, location, &self.global_state);

        future::ready(node_state).boxed()
    }

    fn add_metadata(&self, metadata: HashMap<String, String>) -> BoxFuture<'_, miette::Result<()>> {
        future::ready(add_run_metadata(
            self.run_id,
            self.attempt,
            metadata,
            &self.global_state,
        ))
        .boxed()
    }

    fn read_metadata(&self) -> BoxFuture<'_, miette::Result<HashMap<String, String>>> {
        future::ready(read_run_metadata(
            self.run_id,
            self.attempt,
            &self.global_state,
        ))
        .boxed()
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

    fn handle_node_event(&self, event: NodeEvent, send_update: &mut bool) -> miette::Result<()> {
        let attempt = self.attempt.try_into().into_diagnostic()?;
        let now = Utc::now().naive_utc();
        let mut node_outputs = HashMap::new();
        let node_updates = event
            .locs
            .into_iter()
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
                                node_outputs.insert(
                                    loc.clone(),
                                    NewNodeOutput {
                                        name: port.clone(),
                                        asset_kind: asset_spec.kind.to_string(),
                                        storage_name: asset_spec.storage_name.clone(),
                                        asset_key: asset_spec.asset_key.to_string(),
                                    },
                                );
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
        *send_update = update_node_state(
            &self.global_state,
            &self.run_id.to_string(),
            attempt,
            node_updates,
            node_outputs,
        )?;
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
    #[tokio::test]
    async fn read_location_returns_default() -> miette::Result<()> {
        let runtime_state = SqliteRuntimeState::new_in_memory();

        let run_id = Uuid::now_v7();
        let attempt = 0;
        let workflow_state = runtime_state.workflow_state(run_id, attempt);

        let node_state = workflow_state.read(&Location::root()).await?;

        assert_eq!(node_state, NodeState::default());

        Ok(())
    }

    /// Test that we can write and listen for updates.
    #[tokio::test]
    async fn write_and_listen_for_updates() -> miette::Result<()> {
        let runtime_state = SqliteRuntimeState::new_in_memory();

        let stream = runtime_state.listen()?;

        let run_id = Uuid::now_v7();
        let attempt = 1;
        let workflow_state = runtime_state.workflow_state(run_id, attempt);

        workflow_state
            .write(Event::Node(NodeEvent {
                locs: vec![Location::root()],
                status: NodeStatus::Scheduled,
            }))
            .await?;

        let updated = stream.take(1).collect::<Vec<_>>().await;
        assert_eq!(updated.len(), 1);
        assert_eq!(
            updated[0],
            RunAttemptUpdated {
                run_id,
                attempt,
                stopped: false
            }
        );

        Ok(())
    }

    /// Test that we can read and write workflow run state.
    #[tokio::test]
    async fn write_and_read() -> miette::Result<()> {
        let runtime_state = SqliteRuntimeState::new_in_memory();

        let run_id = Uuid::now_v7();
        let attempt = 2;
        let workflow_state = runtime_state.workflow_state(run_id, attempt);

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
    #[tokio::test]
    async fn write_and_read_outputs() -> miette::Result<()> {
        let runtime_state = SqliteRuntimeState::new_in_memory();

        let run_id = Uuid::now_v7();
        let attempt = 2;
        let workflow_state = runtime_state.workflow_state(run_id, attempt);

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
    #[tokio::test]
    async fn write_and_read_map_completed() -> miette::Result<()> {
        let runtime_state = SqliteRuntimeState::new_in_memory();

        let run_id = Uuid::now_v7();
        let attempt = 2;
        let workflow_state = runtime_state.workflow_state(run_id, attempt);

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
    #[tokio::test]
    async fn write_and_read_metadata() -> miette::Result<()> {
        let runtime_state = SqliteRuntimeState::new_in_memory();

        let run_id = Uuid::now_v7();
        let attempt = 3;
        let workflow_state = runtime_state.workflow_state(run_id, attempt);

        let metadata = HashMap::from_iter([("foo".to_string(), "bar".to_string())]);
        workflow_state.add_metadata(metadata.clone()).await?;

        let read_metadata = workflow_state.read_metadata().await?;

        assert_eq!(metadata, read_metadata);

        Ok(())
    }
    /// Test that metadata we write gets merged.
    #[tokio::test]
    async fn merge_metadata() -> miette::Result<()> {
        let runtime_state = SqliteRuntimeState::new_in_memory();

        let run_id = Uuid::now_v7();
        let attempt = 4;
        let workflow_state = runtime_state.workflow_state(run_id, attempt);

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
