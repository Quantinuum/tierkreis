/*!
The runtime module defines the entrypoint to running Workflows.
*/
use std::{
    collections::HashMap,
    hash::BuildHasher,
    path::Path,
    sync::{Arc, RwLock},
};

use miette::{IntoDiagnostic, miette};
use uuid::Uuid;

use crate::{
    asset_storage::{
        AssetStorage, AssetStorageRegistry, FileAssetStorage, InMemoryStorage,
        load_assets, save_assets,
    },
    executor::{Executor, InMemoryExecutor, SubprocessExecutor},
    graph::WorkflowGraph,
    location::Location,
    orchestrator::{OrchestrationContext, Orchestrator},
    state::{InMemoryRuntimeState, RuntimeState, SqliteRuntimeState, WorkflowRunState},
    updater::Updater,
};

struct Runtime<RS: RuntimeState> {
    orchestrator: Orchestrator,
    state: RS,
    asset_storage_registry: AssetStorageRegistry,
}

impl Runtime<SqliteRuntimeState> {
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
            Box::new(InMemoryExecutor::try_new(
                &asset_storage_registry,
                "memory",
            )?),
        );
        executor_registry.insert(
            "subprocess".to_string(),
            Box::new(SubprocessExecutor::try_new(
                &asset_storage_registry,
                "file",
                "file",
            )?),
        );
        let executor_registry = Arc::new(executor_registry);

        let orchestrator = Orchestrator::try_new(
            &asset_storage_registry,
            &executor_registry,
            "file",
            "subprocess",
        )?;

        let runtime_state = SqliteRuntimeState::try_new().await?;

        Ok(Self {
            orchestrator,
            state: runtime_state,
            asset_storage_registry,
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
            Box::new(InMemoryExecutor::try_new(
                &asset_storage_registry,
                "memory",
            )?),
        );
        let executor_registry = Arc::new(executor_registry);

        let orchestrator = Orchestrator::try_new(
            &asset_storage_registry,
            &executor_registry,
            "memory",
            "memory",
        )?;

        let runtime_state = SqliteRuntimeState::try_new_in_memory().await?;

        Ok(Self {
            orchestrator,
            state: runtime_state,
            asset_storage_registry,
        })
    }
}

impl Runtime<InMemoryRuntimeState> {
    // TODO: Add a from_config function to build a Runtime from a configuration file.
    #[allow(dead_code)]
    fn memory() -> miette::Result<Self> {
        let mut asset_storage_registry: HashMap<String, Box<dyn AssetStorage>> = HashMap::new();
        let memory_storage = InMemoryStorage::new();
        asset_storage_registry.insert("memory".to_string(), Box::new(memory_storage));

        let asset_storage_registry = Arc::new(RwLock::new(asset_storage_registry));

        let mut executor_registry: HashMap<String, Box<dyn Executor>> = HashMap::new();

        executor_registry.insert(
            "memory".to_string(),
            Box::new(InMemoryExecutor::try_new(
                &asset_storage_registry,
                "memory",
            )?),
        );
        let executor_registry = Arc::new(executor_registry);

        let orchestrator = Orchestrator::try_new(
            &asset_storage_registry,
            &executor_registry,
            "memory",
            "memory",
        )?;

        let runtime_state = InMemoryRuntimeState::new();

        Ok(Self {
            orchestrator,
            state: runtime_state,
            asset_storage_registry,
        })
    }
}

impl<RS: RuntimeState> Runtime<RS> {
    async fn start<S: BuildHasher>(
        &mut self,
        workflow_graph: WorkflowGraph,
        inputs: HashMap<String, Vec<u8>, S>,
    ) -> miette::Result<(Uuid, u32)>
    where
        <RS as RuntimeState>::WorkflowRunState: 'static,
    {
        // Persist the workflow graph and get its stable ID.
        let workflow_id = self.state.save_workflow(workflow_graph).await?;

        // Save the raw input bytes as assets and store them with the run.
        let input_assets = save_assets(&self.asset_storage_registry, "memory", inputs)?;
        let workflow_run_state = self
            .state
            .new_workflow_run_state(workflow_id, input_assets)
            .await?;
        let run_id = workflow_run_state.run_id();
        let attempt = workflow_run_state.attempt();

        let workflow_run_state = Arc::new(workflow_run_state);
        let updater = Updater::new(Arc::clone(&workflow_run_state));

        let stream = self.orchestrator.listen()?;
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
                res = updater.process(stream) => {
                    match res {
                        Ok(()) => {},
                        Err(err) => {
                            eprintln!("{err}");
                        }
                    }
                }
            }
        });

        // Load inputs back from state (they are now stored in the DB).
        let inputs = workflow_run_state.load_inputs().await?;
        let graph = Arc::new(self.state.load_workflow(workflow_id).await?);
        let context = OrchestrationContext::new(&workflow_run_state, inputs);
        let actions = self
            .orchestrator
            .build_actions(context.clone(), graph)
            .await?;

        self.orchestrator.perform_actions(actions).await?;

        Ok((run_id, attempt))
    }

    async fn run(&mut self) -> miette::Result<()> {
        let mut state_recv = self.state.listen()?;

        loop {
            let (run_id, attempt) = {
                // WARNING: It's very important that we drop this `updated` ref
                // in order for the orchestrator to be able to send updates later on
                // as this channel uses a RW lock that is held as long as this ref exists.
                //
                // See: https://github.com/tokio-rs/tokio/issues/4246
                let updated = state_recv.borrow_and_update();
                if updated.stopped {
                    break;
                }
                (updated.run_id, updated.attempt)
            };
            let workflow_run_state = self.state.load_workflow_run_state(run_id, attempt).await?;
            let workflow_run_state = Arc::new(workflow_run_state);

            // Load the workflow graph and inputs from the persisted state.
            let workflow_id = workflow_run_state.workflow_id();
            let graph = Arc::new(self.state.load_workflow(workflow_id).await?);
            let inputs = workflow_run_state.load_inputs().await?;

            let context = OrchestrationContext::new(&workflow_run_state, inputs);
            let actions = self.orchestrator.build_actions(context, graph).await?;
            self.orchestrator.perform_actions(actions).await?;
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

        // Load the workflow graph from state to locate the output node.
        let workflow_id = workflow_run_state.workflow_id();
        let graph = self.state.load_workflow(workflow_id).await?;

        let output_state = workflow_run_state
            .read(&Location::from_node_index_iter([graph.output_idx()]))
            .await?;

        let outputs = load_assets(
            &self.asset_storage_registry,
            &output_state
                .outputs
                .ok_or_else(|| miette!("No output values on Output node."))?,
        )?;

        Ok(outputs)
    }
}

#[tokio::main]
pub(crate) async fn run_workflow_in_memory<S: BuildHasher>(
    workflow_graph: WorkflowGraph,
    inputs: HashMap<String, Vec<u8>, S>,
) -> miette::Result<HashMap<String, Vec<u8>>> {
    let mut runtime = Runtime::sqlite_memory().await?;
    let (run_id, attempt) = runtime.start(workflow_graph, inputs).await?;
    runtime.run().await?;
    let outputs = runtime.outputs(run_id, attempt).await?;

    Ok(outputs)
}
