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
        AssetSpec, AssetStorage, AssetStorageRegistry, FileAssetStorage, InMemoryStorage,
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

    // TODO: Hack to work around not storing graphs yet.
    workflow_graph: Option<Arc<WorkflowGraph>>,
    // TODO: Hack to work around not storing inputs yet.
    inputs: HashMap<String, AssetSpec>,
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
            workflow_graph: None,
            inputs: HashMap::new(),
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
            workflow_graph: None,
            inputs: HashMap::new(),
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
            workflow_graph: None,
            inputs: HashMap::new(),
        })
    }
}

impl<RS: RuntimeState> Runtime<RS> {
    async fn start<S: BuildHasher>(
        &mut self,
        // TODO: Take a workflow ID and load the graph instead of passing it
        //workflow_id: Uuid,
        workflow_graph: WorkflowGraph,
        inputs: HashMap<String, Vec<u8>, S>,
    ) -> miette::Result<(Uuid, u32)>
    where
        <RS as RuntimeState>::WorkflowRunState: 'static,
    {
        let run_id = Uuid::now_v7();
        let attempt = 0;

        let workflow_run_state = self.state.load_workflow_run_state(run_id, attempt).await?;
        let workflow_run_state = Arc::new(workflow_run_state);
        let updater = Updater::new(Arc::clone(&workflow_run_state));

        // TODO: Hack to work around not having workflow run ids in events.
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

        let inputs = save_assets(&self.asset_storage_registry, "memory", inputs)?;
        self.inputs.clone_from(&inputs);
        // TODO: Maybe inputs should be part of the workflow run state?
        let context = OrchestrationContext::new(&workflow_run_state, inputs);
        let workflow_graph = Arc::new(workflow_graph);
        let actions = self
            .orchestrator
            .build_actions(context.clone(), Arc::clone(&workflow_graph))
            .await?;
        self.workflow_graph = Some(workflow_graph);

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

            // TODO: Handle inputs better here.
            let context = OrchestrationContext::new(&workflow_run_state, self.inputs.clone());

            let actions = self
                .orchestrator
                .build_actions(context.clone(), self.workflow_graph.clone().unwrap())
                .await?;
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

        let output_state = workflow_run_state
            .read(&Location::from_node_index_iter([self
                .workflow_graph
                .as_ref()
                .unwrap()
                .output_idx()]))
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
