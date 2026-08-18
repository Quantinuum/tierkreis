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
        config.runtime_state = RuntimeStateConfig::Sqlite { memory: true };
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
    Sqlite { memory: bool },
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
        let runtime_state: Arc<dyn RuntimeState> = match config.runtime_state {
            RuntimeStateConfig::Memory {} => Arc::new(InMemoryRuntimeState::new()),
            RuntimeStateConfig::Sqlite { memory: true } => {
                Arc::new(SqliteRuntimeState::try_new_in_memory().await?)
            }
            RuntimeStateConfig::Sqlite { memory: false } => {
                Arc::new(SqliteRuntimeState::try_new().await?)
            }
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
                                NodeStatus::Queued => {
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
        let _task = tokio::spawn(async move {
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
        });

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
