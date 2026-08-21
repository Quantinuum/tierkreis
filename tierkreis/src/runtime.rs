/*!
The runtime module defines the entrypoint to running Workflows.
*/
use std::{collections::HashMap, env::home_dir, hash::BuildHasher, path::PathBuf, sync::Arc};

use futures::{Stream, StreamExt};
use miette::{IntoDiagnostic, miette};
use serde::Deserialize;
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::{
    asset_storage::{
        AssetStorage, AssetStorageRegistry, FileAssetStorage, InMemoryStorage, load_assets,
        save_assets,
    },
    event::{NodeEvent, NodeStatus, RuntimeEvent, WorkflowRunEvent},
    executor::{
        Executor, ExecutorRegistry, InMemoryExecutor, SubprocessExecutor,
        nexus::{NexusClientConfig, NexusExecutor},
    },
    graph::WorkflowGraph,
    location::Location,
    monitoring::{LoggingConfig, flush_logs, init_logging_and_tracing},
    orchestrator::{OrchestrationContext, Orchestrator},
    state::{InMemoryRuntimeState, RuntimeState, SqliteRuntimeState},
};

/// `RuntimeConfig` defines the configuration for the runtime
#[derive(Deserialize)]
pub struct RuntimeConfig {
    asset_storage: HashMap<String, AssetStorageConfig>,
    executors: HashMap<String, ExecutorConfig>,
    runtime_state: RuntimeStateConfig,

    default_storage_name: String,
    default_executor_name: String,
    logging_config: Option<LoggingConfig>,
}

impl RuntimeConfig {
    fn memory() -> Self {
        RuntimeConfig {
            asset_storage: [("memory".to_string(), AssetStorageConfig::Memory {})]
                .into_iter()
                .collect(),
            executors: [(
                "memory".to_string(),
                ExecutorConfig::Memory {
                    output_storage_name: "memory".to_string(),
                },
            )]
            .into_iter()
            .collect(),
            runtime_state: RuntimeStateConfig::Memory {},
            default_storage_name: "memory".to_string(),
            default_executor_name: "memory".to_string(),
            logging_config: Some(LoggingConfig::default()),
        }
    }

    fn sqlite_memory() -> Self {
        let mut config = Self::memory();
        config.runtime_state = RuntimeStateConfig::Sqlite {
            memory: true,
            url: None,
        };
        config
    }
}

impl Default for RuntimeConfig {
    fn default() -> Self {
        let tierkreis_dir = home_dir()
            .unwrap_or_else(|| "/tmp".into())
            .join(".tierkreis");

        let asset_dir = tierkreis_dir.join("assets");
        RuntimeConfig {
            asset_storage: [
                ("memory".to_string(), AssetStorageConfig::Memory {}),
                ("file".to_string(), AssetStorageConfig::File { asset_dir }),
            ]
            .into_iter()
            .collect(),
            executors: [
                (
                    "memory".to_string(),
                    ExecutorConfig::Memory {
                        output_storage_name: "memory".to_string(),
                    },
                ),
                (
                    "subprocess".to_string(),
                    ExecutorConfig::Subprocess {
                        subprocess_storage_name: "file".to_string(),
                        output_storage_name: "file".to_string(),
                    },
                ),
            ]
            .into_iter()
            .collect(),
            runtime_state: RuntimeStateConfig::Memory {},
            default_storage_name: "file".to_string(),
            default_executor_name: "subprocess".to_string(),
            logging_config: Some(LoggingConfig::default()),
        }
    }
}

#[derive(Deserialize)]
enum AssetStorageConfig {
    Memory {},
    File { asset_dir: PathBuf },
}

#[derive(Deserialize)]
enum ExecutorConfig {
    Memory {
        output_storage_name: String,
    },
    Subprocess {
        subprocess_storage_name: String,
        output_storage_name: String,
    },
    Nexus {
        client_config: NexusClientConfig,
        output_storage_name: String,
    },
}

#[derive(Deserialize)]
enum RuntimeStateConfig {
    Memory {},
    Sqlite { memory: bool, url: Option<String> },
}

struct Runtime {
    orchestrator: Orchestrator,
    state: Arc<dyn RuntimeState>,
    asset_storage_registry: AssetStorageRegistry,
    default_storage_name: String,

    // Optional Run ID to execute exclusively. Once this run completes the
    // runtime should end execution.
    dedicated_run_id: Option<Uuid>,
}

struct AbortOnDrop(tokio::task::JoinHandle<()>);

impl Drop for AbortOnDrop {
    fn drop(&mut self) {
        self.0.abort();
    }
}

impl Runtime {
    async fn from_config(config: &RuntimeConfig) -> miette::Result<Self> {
        let asset_storage_registry = asset_storage_registry_from_config(config);

        let executor_registry =
            executor_registry_from_config(&asset_storage_registry, config).await?;

        init_logging_and_tracing(config.logging_config.clone());
        let orchestrator = Orchestrator::try_new(
            &asset_storage_registry,
            &executor_registry,
            &config.default_storage_name,
            &config.default_executor_name,
        )
        .await?;
        let runtime_state: Arc<dyn RuntimeState> = match &config.runtime_state {
            RuntimeStateConfig::Memory {} => Arc::new(InMemoryRuntimeState::new()),
            RuntimeStateConfig::Sqlite { memory: true, .. } => {
                Arc::new(SqliteRuntimeState::try_new_in_memory().await?)
            }
            RuntimeStateConfig::Sqlite { memory: false, url } => match url.as_deref() {
                Some(url) => Arc::new(SqliteRuntimeState::try_new_with_url(url).await?),
                None => Arc::new(SqliteRuntimeState::try_new().await?),
            },
        };
        tracing::info!("Starting Tierkreis runtime");
        Ok(Self {
            orchestrator,
            state: runtime_state,
            asset_storage_registry,
            default_storage_name: config.default_storage_name.clone(),
            dedicated_run_id: None,
        })
    }

    async fn save_workflow(&self, workflow_graph: WorkflowGraph) -> miette::Result<Uuid> {
        self.state.save_workflow(workflow_graph).await
    }

    async fn start_new_run<S: BuildHasher>(
        &mut self,
        workflow_id: Uuid,
        inputs: HashMap<String, Vec<u8>, S>,
    ) -> miette::Result<(Uuid, u32)> {
        let inputs = save_assets(
            &self.asset_storage_registry,
            &self.default_storage_name,
            inputs,
        )
        .await?;
        let workflow_run_state = self
            .state
            .new_workflow_run_state(workflow_id, inputs)
            .await?;
        let attempt = workflow_run_state.attempt();
        let run_id = workflow_run_state.run_id();
        tracing::info!(workflow_id = %workflow_id.to_string(), run_id = %run_id.to_string(), attempt = attempt, "Starting new run attempt");
        Ok((run_id, attempt))
    }

    #[allow(clippy::too_many_lines)]
    async fn process_events(
        state: Arc<dyn RuntimeState>,
        mut stream: impl Stream<Item = RuntimeEvent> + Unpin,
    ) -> miette::Result<()> {
        while let Some(event) = stream.next().await {
            match event {
                RuntimeEvent::WorkflowRun {
                    workflow_run_id,
                    attempt,
                    event,
                } => {
                    let workflow_state = state
                        .load_workflow_run_state(workflow_run_id, attempt)
                        .await?;
                    let workflow_id = workflow_state.workflow_id().to_string();
                    match &event {
                        WorkflowRunEvent::Started {} => {
                            tracing::info!(workflow_id = %workflow_id, run_id = %workflow_run_id, attempt, "workflow started");
                        }
                        WorkflowRunEvent::Queued {} => {
                            tracing::info!(workflow_id = %workflow_id, run_id = %workflow_run_id, attempt, "workflow queued");
                        }
                        WorkflowRunEvent::Completed {} => {
                            tracing::info!(workflow_id = %workflow_id, run_id = %workflow_run_id, attempt, "workflow completed");
                        }
                        WorkflowRunEvent::Errored {} => {
                            tracing::error!(workflow_id = %workflow_id, run_id = %workflow_run_id, attempt, "Workflow errored");
                        }
                        WorkflowRunEvent::Cancelled {} => {
                            tracing::error!(workflow_id = %workflow_id, run_id = %workflow_run_id, attempt, "Workflow cancelled");
                        }
                        WorkflowRunEvent::NodeEvent(NodeEvent { locs, status }) => {
                            let locs = locs.iter().map(ToString::to_string).collect::<Vec<_>>();
                            // Whitespaces are added to align for LogFormat::Compact
                            match &status {
                                NodeStatus::Scheduled => {
                                    tracing::info!(
                                        target: "tierkreis::events",
                                        workflow_id = %workflow_id,
                                        run_id = %workflow_run_id,
                                        attempt,
                                        ?locs,
                                        "node scheduled"
                                    );
                                }
                                NodeStatus::Queued { .. } => {
                                    tracing::info!(
                                        target: "tierkreis::events",
                                        workflow_id = %workflow_id,
                                        run_id = %workflow_run_id,
                                        attempt,
                                        ?locs,
                                        "node queued   "
                                    );
                                }
                                NodeStatus::Running { state_update } => {
                                    tracing::info!(
                                        target: "tierkreis::events",
                                        workflow_id = %workflow_id,
                                        run_id = %workflow_run_id,
                                        attempt,
                                        ?locs,
                                        ?state_update,
                                        "node running  "
                                    );
                                }
                                NodeStatus::Complete { .. } => {
                                    tracing::info!(
                                        target: "tierkreis::events",
                                        workflow_id = %workflow_id,
                                        run_id = %workflow_run_id,
                                        attempt,
                                        ?locs,
                                        "node completed"
                                    );
                                }
                                NodeStatus::Error { error, .. } => {
                                    tracing::error!(
                                        target: "tierkreis::events",
                                        workflow_id = %workflow_id,
                                        run_id = %workflow_run_id,
                                        attempt,
                                        ?locs,
                                        ?error,
                                        "node errored  "
                                    );
                                }
                                NodeStatus::Cancelled => {
                                    tracing::error!(
                                        target: "tierkreis::events",
                                        workflow_id = %workflow_id,
                                        run_id = %workflow_run_id,
                                        attempt,
                                        ?locs,
                                        "node cancelled"
                                    );
                                }
                            }
                        }
                    }
                    workflow_state.write(event).await?;
                }
            }
        }
        Ok(())
    }

    async fn run(&mut self) -> miette::Result<()> {
        let stream = self.orchestrator.listen()?;
        let state = self.state.clone();
        let _task = AbortOnDrop(tokio::spawn(async move {
            tokio::select! {
                sig = tokio::signal::ctrl_c() => {
                    match sig {
                        Ok(()) => {
                            tracing::info!("Received ctrl-c signal, shutting down runtime");
                            flush_logs();
                            std::process::exit(130)},
                        Err(err) => {
                            tracing::error!("Error while waiting for ctrl-c signal: {err}");
                            flush_logs();
                            eprintln!("{err}");
                            std::process::exit(1);
                        }
                    }
                }
                res = Self::process_events(state, stream) => {
                    match res {
                        Ok(()) => {},
                        Err(err) => {
                            tracing::error!("Error while processing events: {err}");
                            eprintln!("{err}");
                        }
                    }
                }
            }
        }));

        let mut state_recv = self.state.listen();

        loop {
            let active_runs: Vec<(Uuid, u32)> = {
                // WARNING: It's very important that we drop this `updated` ref
                // in order for the orchestrator to be able to send updates later on
                // as this channel uses a RW lock that is held as long as this ref exists.
                //
                // See: https://github.com/tokio-rs/tokio/issues/4246
                let updated = state_recv.borrow_and_update();
                if let Some(terminate_on_complete) = self.dedicated_run_id {
                    if updated.active_runs.contains(&(terminate_on_complete, 0)) {
                        vec![(terminate_on_complete, 0)]
                    } else {
                        break;
                    }
                } else {
                    updated.active_runs.iter().copied().collect()
                }
            };
            for (run_id, attempt) in active_runs {
                let workflow_run_state =
                    self.state.load_workflow_run_state(run_id, attempt).await?;
                let workflow_id = workflow_run_state.workflow_id();
                let workflow_graph = self.state.load_workflow(workflow_id).await?;
                let inputs = workflow_run_state.load_inputs().await?;

                let workflow_run_state = Arc::new(workflow_run_state);
                let workflow_graph = Arc::new(workflow_graph);

                let context = OrchestrationContext::new(&workflow_run_state, inputs);

                let actions = self
                    .orchestrator
                    .build_actions(context, workflow_graph)
                    .await?;
                self.orchestrator
                    .perform_actions(run_id, attempt, actions)
                    .await?;
            }
            state_recv.changed().await.into_diagnostic()?;
        }
        tracing::info!("Runtime exiting, shutting down logging");
        flush_logs();
        Ok(())
    }

    async fn outputs(
        &mut self,
        run_id: Uuid,
        attempt: u32,
    ) -> miette::Result<HashMap<String, Vec<u8>>> {
        let workflow_run_state = self.state.load_workflow_run_state(run_id, attempt).await?;
        let workflow_id = workflow_run_state.workflow_id();
        // TODO: Use LRU cache for workflows or similar here?
        let workflow_graph = self.state.load_workflow(workflow_id).await?;

        let output_state = workflow_run_state
            .read(&Location::from_node_index_iter([
                workflow_graph.output_idx()
            ]))
            .await?;

        let outputs = load_assets(
            &self.asset_storage_registry,
            &output_state
                .outputs
                .ok_or_else(|| miette!("No output values on Output node."))?,
        )
        .await?;

        Ok(outputs)
    }
}

async fn executor_registry_from_config(
    asset_storage_registry: &AssetStorageRegistry,
    config: &RuntimeConfig,
) -> Result<ExecutorRegistry, miette::Error> {
    let mut executor_registry: HashMap<String, Box<dyn Executor>> = HashMap::new();
    for (executor_name, executor_config) in &config.executors {
        match executor_config {
            ExecutorConfig::Memory {
                output_storage_name,
            } => executor_registry.insert(
                executor_name.clone(),
                Box::new(
                    InMemoryExecutor::try_new(asset_storage_registry, output_storage_name).await?,
                ),
            ),
            ExecutorConfig::Subprocess {
                subprocess_storage_name,
                output_storage_name,
            } => executor_registry.insert(
                executor_name.clone(),
                Box::new(
                    SubprocessExecutor::try_new(
                        asset_storage_registry,
                        subprocess_storage_name,
                        output_storage_name,
                    )
                    .await?,
                ),
            ),
            ExecutorConfig::Nexus {
                client_config,
                output_storage_name,
            } => executor_registry.insert(
                executor_name.clone(),
                Box::new(
                    NexusExecutor::try_new(
                        client_config,
                        asset_storage_registry,
                        output_storage_name,
                    )
                    .await?,
                ),
            ),
        };
    }
    let executor_registry = Arc::new(executor_registry);
    Ok(executor_registry)
}

/// Create an [`AssetStorageRegistry`] from the given [`RuntimeConfig`].
#[must_use]
pub fn asset_storage_registry_from_config(config: &RuntimeConfig) -> AssetStorageRegistry {
    let mut asset_storage_registry: HashMap<String, Box<dyn AssetStorage>> = HashMap::new();
    for (asset_storage_name, asset_storage_config) in &config.asset_storage {
        match asset_storage_config {
            AssetStorageConfig::Memory {} => asset_storage_registry
                .insert(asset_storage_name.clone(), Box::new(InMemoryStorage::new())),
            AssetStorageConfig::File { asset_dir: parent } => asset_storage_registry.insert(
                asset_storage_name.clone(),
                Box::new(FileAssetStorage::new(parent)),
            ),
        };
    }
    Arc::new(RwLock::new(asset_storage_registry))
}

#[tokio::main]
pub(crate) async fn run_workflow_in_memory<S: BuildHasher>(
    workflow_graph: WorkflowGraph,
    inputs: HashMap<String, Vec<u8>, S>,
) -> miette::Result<HashMap<String, Vec<u8>>> {
    let mut runtime = Runtime::from_config(&RuntimeConfig::sqlite_memory()).await?;

    let workflow_id = runtime.save_workflow(workflow_graph).await?;
    let (run_id, attempt) = runtime.start_new_run(workflow_id, inputs).await?;

    runtime.dedicated_run_id = Some(run_id);
    runtime.run().await?;

    let outputs = runtime.outputs(run_id, attempt).await?;

    flush_logs();

    Ok(outputs)
}

/// Start the runtime until cancelled.
///
/// # Errors
///
/// Will return Err if the [`Runtime`] cannot be configured or if an unrecoverable
/// error happens while running.
///
/// # Panics
///
/// Will panic if there is already a tokio runtime active.
#[tokio::main]
pub async fn exec() -> miette::Result<()> {
    let mut runtime = Runtime::from_config(&RuntimeConfig::default()).await?;
    runtime.run().await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::builder::{constant, input, link, output, task, workflow};
    use futures::future::{AbortHandle, Abortable};
    use tempfile::NamedTempFile;

    #[tokio::test(flavor = "multi_thread")]
    async fn run_can_resume_after_runtime_is_terminated() -> miette::Result<()> {
        let database_file = NamedTempFile::new().into_diagnostic()?;
        let database_url = database_file.path().to_string_lossy().into_owned();
        let mut config = RuntimeConfig::memory();
        config.runtime_state = RuntimeStateConfig::Sqlite {
            memory: false,
            url: Some(database_url),
        };

        let mut workflow_graph = workflow(["result"]);
        let delay = input(&mut workflow_graph, "delay_seconds");
        let task = task(
            &mut workflow_graph,
            "builtin",
            "sleep",
            ["delay_seconds"],
            ["value"],
        );
        let out = output(&workflow_graph, "result");
        link(&mut workflow_graph, delay, (task, "delay_seconds"))?;
        link(&mut workflow_graph, (task, "value"), out)?;

        let mut runtime = Runtime::from_config(&config).await?;
        let workflow_id = runtime.save_workflow(workflow_graph.clone()).await?;
        let inputs = HashMap::from([(
            "delay_seconds".to_string(),
            serde_json::to_vec(&1).into_diagnostic()?,
        )]);
        let (run_id, attempt) = runtime.start_new_run(workflow_id, inputs).await?;
        runtime.dedicated_run_id = Some(run_id);
        let workflow_run_state = runtime
            .state
            .load_workflow_run_state(run_id, attempt)
            .await?;
        let task_location = Location::root().with_node(task);

        let (abort_handle, abort_registration) = AbortHandle::new_pair();
        let run = Abortable::new(runtime.run(), abort_registration);
        tokio::pin!(run);
        let wait_for_completion = async {
            loop {
                if workflow_run_state
                    .read(&task_location)
                    .await?
                    .outputs
                    .is_some()
                {
                    return Ok::<(), miette::Report>(());
                }
                tokio::task::yield_now().await;
            }
        };
        tokio::pin!(wait_for_completion);
        tokio::select! {
            result = &mut run => panic!("runtime terminated before interruption: {result:?}"),
            result = &mut wait_for_completion => result?,
        }
        abort_handle.abort();
        assert!(run.await.is_err(), "the first runtime was not terminated");

        let mut resumed_runtime = Runtime::from_config(&config).await?;
        resumed_runtime.dedicated_run_id = Some(run_id);
        resumed_runtime.run().await?;

        let resumed_state = resumed_runtime
            .state
            .load_workflow_run_state(run_id, attempt)
            .await?;
        assert!(
            resumed_state
                .read(&Location::root().with_node(workflow_graph.output_idx()))
                .await?
                .outputs
                .is_some()
        );

        Ok(())
    }

    fn empty_hugr() -> miette::Result<hugr::Hugr> {
        use hugr::{
            builder::{FunctionBuilder, HugrBuilder},
            types::Signature,
        };
        let hugr = FunctionBuilder::new("main", Signature::new(vec![], vec![]))
            .into_diagnostic()?
            .finish_hugr()
            .into_diagnostic()?;
        Ok(hugr)
    }

    fn hugr_package_bytes() -> miette::Result<Vec<u8>> {
        use hugr::{
            envelope::{EnvelopeConfig, write_envelope},
            package::Package,
        };
        let package = Package::new([empty_hugr()?]);
        let mut buf = Vec::new();
        write_envelope(&mut buf, &package, EnvelopeConfig::binary()).into_diagnostic()?;
        Ok(buf)
    }

    /// Happy path nexus test app
    #[allow(clippy::too_many_lines)]
    fn nexus_restart_test_app() -> (
        axum::Router,
        tokio::sync::watch::Sender<bool>,
        Arc<std::sync::atomic::AtomicU32>,
    ) {
        use axum::{
            Json, Router,
            extract::{Query, State, WebSocketUpgrade, ws::WebSocket},
        };
        use serde::Deserialize;
        use serde_json::json;
        use tokio::sync::watch;

        async fn send_ws_message(socket: &mut WebSocket, message: &serde_json::Value) {
            let msg = axum::extract::ws::Message::text(
                serde_json::to_string(message).expect("failed to serialize"),
            );
            socket.send(msg).await.expect("failed to send");
        }

        #[derive(Clone)]
        struct AppState {
            recv: watch::Receiver<bool>,
            submission_count: Arc<std::sync::atomic::AtomicU32>,
        }

        async fn handle_test_socket(mut socket: WebSocket, mut recv: watch::Receiver<bool>) {
            send_ws_message(
                &mut socket,
                &json!({"status": "SUBMITTED", "message": "job is submitted"}),
            )
            .await;
            // Only progress towards completion once signalled by the test.
            recv.wait_for(|completed| *completed).await.unwrap();
            send_ws_message(
                &mut socket,
                &json!({"status": "COMPLETED", "message": "job is completed"}),
            )
            .await;
            socket
                .send(axum::extract::ws::Message::Close(None))
                .await
                .unwrap();
        }

        #[derive(Deserialize)]
        struct ChunkParameters {
            chunk_number: u64,
        }

        let (send, recv) = watch::channel(false);
        let submission_count = Arc::new(std::sync::atomic::AtomicU32::new(0));
        let app = Router::new()
            .route("/auth/tokens/refresh", axum::routing::post(|| async {}))
            .route(
                "/api/projects/v1beta2",
                axum::routing::get(|| async {
                    let id = Uuid::now_v7();
                    Json(json!({"data": [{"id": id}]}))
                }),
            )
            .route(
                "/api/hugr/v1beta",
                axum::routing::post(|| async {
                    let id = Uuid::now_v7();
                    Json(json!({"data": {"id": id}}))
                }),
            )
            .route(
                "/api/jobs/v1beta3",
                axum::routing::post(|State(state): State<AppState>| async move {
                    state
                        .submission_count
                        .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                    let id = Uuid::now_v7();
                    Json(json!({"data": {"id": id}}))
                }),
            )
            .route(
                "/api/jobs/v1beta3/{job_id}/attributes/status/ws",
                axum::routing::get(
                    |State(state): State<AppState>, ws: WebSocketUpgrade| async move {
                        let recv = state.recv.clone();
                        ws.on_upgrade(|socket| handle_test_socket(socket, recv))
                    },
                ),
            )
            .route(
                "/api/jobs/v1beta3/{job_id}",
                axum::routing::get(|| async {
                    let id = Uuid::now_v7();
                    Json(json!({
                        "data": {
                            "attributes": {
                                "status": {
                                    "status": "COMPLETED",
                                    "message": "job has completed",
                                },
                                "definition": {
                                    "job_definition_type": "execute_job_definition",
                                    "items": [{"result_id": id}],
                                },
                            },
                        },
                    }))
                }),
            )
            .route(
                "/api/qsys_results/v1beta2/partial/{result_id}",
                axum::routing::get(|parameters: Query<ChunkParameters>| async move {
                    if parameters.chunk_number == 1 {
                        return Err(axum::http::StatusCode::NOT_FOUND);
                    }
                    Ok(Json(json!({
                        "data": {
                            "attributes": {
                                "results": [[]],
                            },
                        },
                    })))
                }),
            )
            .with_state(AppState {
                recv,
                submission_count: submission_count.clone(),
            });

        (app, send, submission_count)
    }

    /// Test that a Task node dispatched to the [`crate::executor::nexus::NexusExecutor`]
    /// is reattached to its Nexus job (rather than resubmitted) after the runtime
    /// is killed and resumed, and that the now-completed job's results are picked up.
    #[allow(clippy::too_many_lines)]
    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn run_can_resume_nexus_task_after_runtime_is_terminated() -> miette::Result<()> {
        use crate::executor::nexus::{NexusClientConfig, client::TLSMode};

        let (app, release_sender, submission_count) = nexus_restart_test_app();
        let server = axum_test::TestServer::builder().http_transport().build(app);
        let url = server.server_address().expect("No server address");

        let token_dir = crate::executor::nexus::client::tests::setup_temp_tokens().await?;
        let client_config = NexusClientConfig {
            tls_mode: TLSMode::None,
            host: format!(
                "{}:{}",
                url.host_str().expect("No host"),
                url.port().expect("No port")
            ),
            token_dir: Some(token_dir.path().to_path_buf()),
        };

        let database_file = NamedTempFile::new().into_diagnostic()?;
        let database_url = database_file.path().to_string_lossy().into_owned();
        let mut config = RuntimeConfig::memory();
        config.executors = HashMap::from([(
            "nexus".to_string(),
            ExecutorConfig::Nexus {
                client_config,
                output_storage_name: "memory".to_string(),
            },
        )]);
        config.default_executor_name = "nexus".to_string();
        config.runtime_state = RuntimeStateConfig::Sqlite {
            memory: false,
            url: Some(database_url),
        };

        let mut workflow_graph = workflow(["result"]);
        let hugr_package = input(&mut workflow_graph, "hugr_package");
        let project_name = constant(&mut workflow_graph, "tkr-demo")?;
        let job_name = constant(&mut workflow_graph, "tkr-example-job")?;
        let n_shots = constant(&mut workflow_graph, 5u64)?;
        let task_node = task(
            &mut workflow_graph,
            "nexus_worker",
            "submit_and_run",
            ["project_name", "job_name", "n_shots", "hugr_package"],
            ["results"],
        );
        let out = output(&workflow_graph, "result");
        link(
            &mut workflow_graph,
            project_name,
            (task_node, "project_name"),
        )?;
        link(&mut workflow_graph, job_name, (task_node, "job_name"))?;
        link(&mut workflow_graph, n_shots, (task_node, "n_shots"))?;
        link(
            &mut workflow_graph,
            hugr_package,
            (task_node, "hugr_package"),
        )?;
        link(&mut workflow_graph, (task_node, "results"), out)?;

        let mut runtime = Runtime::from_config(&config).await?;
        let workflow_id = runtime.save_workflow(workflow_graph.clone()).await?;
        let inputs = HashMap::from([("hugr_package".to_string(), hugr_package_bytes()?)]);
        let (run_id, attempt) = runtime.start_new_run(workflow_id, inputs).await?;
        runtime.dedicated_run_id = Some(run_id);
        let workflow_run_state = runtime
            .state
            .load_workflow_run_state(run_id, attempt)
            .await?;
        let task_location = Location::root().with_node(task_node);
        let mut state_updates = runtime.state.listen();
        let (abort_handle, abort_registration) = AbortHandle::new_pair();
        let run = Abortable::new(runtime.run(), abort_registration);
        tokio::pin!(run);
        // Wait for the first submission, WS sends SUBMITTED
        let wait_for_queued = async {
            loop {
                state_updates.changed().await.into_diagnostic()?;
                if workflow_run_state
                    .read(&task_location)
                    .await?
                    .handle
                    .is_some()
                {
                    return Ok::<(), miette::Report>(());
                }
            }
        };
        tokio::pin!(wait_for_queued);
        tokio::select! {
            result = &mut run => panic!("runtime terminated before interruption: {result:?}"),
            result = &mut wait_for_queued => result?,
        }
        abort_handle.abort();
        // TODO: At this point one monitor tokio::task is still running from the nexus executor.
        // We will spawn a second one with the restart.
        // For the purpose of this test this doesn't matter, but we should fix once we have a
        // better cancel / shutdown mechanism for the runtime.

        assert!(run.await.is_err(), "the first runtime was not terminated");
        // Job Sumission counter should be 1
        assert_eq!(
            submission_count.load(std::sync::atomic::Ordering::SeqCst),
            1
        );
        let job_id_before_resume = workflow_run_state.read(&task_location).await?.handle;

        // Let the mock Nexus job complete, as if it finished while the
        // runtime was down.
        release_sender.send(true).into_diagnostic()?;

        let mut resumed_runtime = Runtime::from_config(&config).await?;
        resumed_runtime.dedicated_run_id = Some(run_id);
        resumed_runtime.run().await?;

        // The resumed run must reattach to the same Nexus job rather than
        // resubmitting.
        assert_eq!(
            submission_count.load(std::sync::atomic::Ordering::SeqCst),
            1
        );
        let resumed_state = resumed_runtime
            .state
            .load_workflow_run_state(run_id, attempt)
            .await?;
        assert_eq!(
            resumed_state.read(&task_location).await?.handle,
            job_id_before_resume
        );

        let outputs = resumed_runtime.outputs(run_id, attempt).await?;
        assert_eq!(
            outputs.get("result").map(Vec::as_slice),
            Some(
                serde_json::to_vec(&serde_json::json!([[]]))
                    .into_diagnostic()?
                    .as_slice()
            )
        );

        Ok(())
    }
}
