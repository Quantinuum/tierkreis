/*!
The runtime module defines the entrypoint to running Workflows.
*/
use std::{collections::HashMap, hash::BuildHasher, path::Path, sync::Arc};

use futures::{Stream, StreamExt};
use miette::{IntoDiagnostic, miette};
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::{
    asset_storage::{
        AssetStorage, AssetStorageRegistry, FileAssetStorage, InMemoryStorage, load_assets,
        save_assets,
    },
    event::RuntimeEvent,
    executor::{Executor, InMemoryExecutor, SubprocessExecutor},
    graph::WorkflowGraph,
    location::Location,
    orchestrator::{OrchestrationContext, Orchestrator},
    state::{InMemoryRuntimeState, RuntimeState, SqliteRuntimeState},
};

struct Runtime {
    orchestrator: Orchestrator,
    state: Arc<dyn RuntimeState>,
    asset_storage_registry: AssetStorageRegistry,

    // Optional Run ID to execute exclusively. Once this run completes the
    // runtime should end execution.
    dedicated_run_id: Option<Uuid>,
}

impl Runtime {
    // TODO: Add a from_config function to build a Runtime from a configuration file.
    #[allow(dead_code)]
    async fn persistent(path: &Path) -> miette::Result<Self> {
        let mut asset_storage_registry: HashMap<String, Box<dyn AssetStorage>> = HashMap::new();
        asset_storage_registry.insert("memory".to_string(), Box::new(InMemoryStorage::new()));
        asset_storage_registry.insert("file".to_string(), Box::new(FileAssetStorage::new(path)));
        let asset_storage_registry = Arc::new(RwLock::new(asset_storage_registry));

        let mut executor_registry: HashMap<String, Box<dyn Executor>> = HashMap::new();
        executor_registry.insert(
            "memory".to_string(),
            Box::new(InMemoryExecutor::try_new(&asset_storage_registry, "memory").await?),
        );
        executor_registry.insert(
            "subprocess".to_string(),
            Box::new(SubprocessExecutor::try_new(&asset_storage_registry, "file", "file").await?),
        );
        let executor_registry = Arc::new(executor_registry);

        let orchestrator = Orchestrator::try_new(
            &asset_storage_registry,
            &executor_registry,
            "file",
            "subprocess",
        )
        .await?;

        let runtime_state = SqliteRuntimeState::try_new().await?;

        Ok(Self {
            orchestrator,
            state: Arc::new(runtime_state),
            asset_storage_registry,
            dedicated_run_id: None,
        })
    }

    // TODO: Add a from_config function to build a Runtime from a configuration file.
    #[allow(dead_code)]
    async fn sqlite_memory() -> miette::Result<Self> {
        let mut asset_storage_registry: HashMap<String, Box<dyn AssetStorage>> = HashMap::new();
        let memory_storage = InMemoryStorage::new();
        asset_storage_registry.insert("memory".to_string(), Box::new(memory_storage));

        let asset_storage_registry = Arc::new(RwLock::new(asset_storage_registry));

        let mut executor_registry: HashMap<String, Box<dyn Executor>> = HashMap::new();

        executor_registry.insert(
            "memory".to_string(),
            Box::new(InMemoryExecutor::try_new(&asset_storage_registry, "memory").await?),
        );
        let executor_registry = Arc::new(executor_registry);

        let orchestrator = Orchestrator::try_new(
            &asset_storage_registry,
            &executor_registry,
            "memory",
            "memory",
        )
        .await?;

        let runtime_state = SqliteRuntimeState::try_new_in_memory().await?;

        Ok(Self {
            orchestrator,
            state: Arc::new(runtime_state),
            asset_storage_registry,
            dedicated_run_id: None,
        })
    }

    // TODO: Add a from_config function to build a Runtime from a configuration file.
    #[allow(dead_code)]
    async fn memory() -> miette::Result<Self> {
        let mut asset_storage_registry: HashMap<String, Box<dyn AssetStorage>> = HashMap::new();
        let memory_storage = InMemoryStorage::new();
        asset_storage_registry.insert("memory".to_string(), Box::new(memory_storage));

        let asset_storage_registry = Arc::new(RwLock::new(asset_storage_registry));

        let mut executor_registry: HashMap<String, Box<dyn Executor>> = HashMap::new();

        executor_registry.insert(
            "memory".to_string(),
            Box::new(InMemoryExecutor::try_new(&asset_storage_registry, "memory").await?),
        );
        let executor_registry = Arc::new(executor_registry);

        let orchestrator = Orchestrator::try_new(
            &asset_storage_registry,
            &executor_registry,
            "memory",
            "memory",
        )
        .await?;

        let runtime_state = InMemoryRuntimeState::new();

        Ok(Self {
            orchestrator,
            state: Arc::new(runtime_state),
            asset_storage_registry,
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
        let inputs = save_assets(&self.asset_storage_registry, "memory", inputs).await?;
        let workflow_run_state = self
            .state
            .new_workflow_run_state(workflow_id, inputs)
            .await?;

        Ok((workflow_run_state.run_id(), workflow_run_state.attempt()))
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
                        Ok(()) => std::process::exit(130),
                        Err(err) => {
                            eprintln!("{err}");
                            std::process::exit(1);
                        }
                    }
                }
                res = Self::process_events(state, stream) => {
                    match res {
                        Ok(()) => {},
                        Err(err) => {
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

#[tokio::main]
pub(crate) async fn run_workflow_in_memory<S: BuildHasher>(
    workflow_graph: WorkflowGraph,
    inputs: HashMap<String, Vec<u8>, S>,
) -> miette::Result<HashMap<String, Vec<u8>>> {
    let mut runtime = Runtime::sqlite_memory().await?;

    let workflow_id = runtime.save_workflow(workflow_graph).await?;
    let (run_id, attempt) = runtime.start_new_run(workflow_id, inputs).await?;

    runtime.dedicated_run_id = Some(run_id);
    runtime.run().await?;

    let outputs = runtime.outputs(run_id, attempt).await?;

    Ok(outputs)
}
