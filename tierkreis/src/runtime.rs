/*!
The runtime module defines the entrypoint to running Workflows.
*/
use std::{
    collections::HashMap,
    ffi::{CStr, CString},
    fs::File,
    hash::BuildHasher,
    io::Read,
    path::Path,
    sync::{Arc, RwLock},
};

use futures::StreamExt;
use miette::{
    Error, IntoDiagnostic, LabeledSpan, MietteDiagnostic, NamedSource, SourceOffset, SourceSpan,
    miette,
};
use pyo3::{exceptions::PySyntaxError, prelude::*};
use uuid::Uuid;

use crate::{
    asset_storage::{
        AssetSpec, AssetStorage, AssetStorageRegistry, InMemoryStorage, load_assets, save_assets,
    },
    executor::{Executor, InMemoryExecutor},
    graph::{LegacyWorkflowGraph, WorkflowGraph},
    location::Location,
    orchestrator::{OrchestrationContext, Orchestrator},
    state::{InMemoryRuntimeState, RuntimeState},
    updater::Updater,
};

/// Utility macro is make nicer diagnostics and return early when handling python exceptions.
macro_rules! getattr_or_early_return {
    ($obj:ident, $attr:expr) => {{
        let attr_any = match $obj.getattr($attr).into_diagnostic() {
            Ok(attr) => attr,
            Err(err) => return err,
        };

        let attr = match attr_any.extract().into_diagnostic() {
            Ok(attr) => attr,
            Err(err) => return err,
        };

        attr
    }};
}

struct Runtime {
    orchestrator: Orchestrator,
    state: Box<dyn RuntimeState>,
    asset_storage_registry: AssetStorageRegistry,

    // TODO: Hack to work around not storing graphs yet.
    workflow_graph: Option<Arc<WorkflowGraph>>,
    // TODO: Hack to work around not storing inputs yet.
    inputs: HashMap<String, AssetSpec>,
}

impl Runtime {
    // TODO: Add a from_config function to build a Runtime from a configuration file.
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

        let runtime_state = Box::new(InMemoryRuntimeState::new());

        Ok(Self {
            orchestrator,
            state: runtime_state,
            asset_storage_registry,
            workflow_graph: None,
            inputs: HashMap::new(),
        })
    }

    async fn start<S: BuildHasher>(
        &mut self,
        // TODO: Take a workflow ID and load the graph instead of passing it
        //workflow_id: Uuid,
        workflow_graph: WorkflowGraph,
        inputs: HashMap<String, Vec<u8>, S>,
    ) -> miette::Result<(Uuid, u32)> {
        let run_id = Uuid::now_v7();
        let attempt = 0;

        let workflow_state = self.state.workflow_state(run_id, attempt);
        let updater = Updater::new(Arc::clone(&workflow_state));

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
        let context = OrchestrationContext::new(&workflow_state, inputs);
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
        let state_events = self.state.listen()?;
        let mut state_chunks = state_events.ready_chunks(32);

        while let Some(chunk) = state_chunks.next().await {
            if chunk.iter().any(|updated| updated.stopped) {
                break;
            }
            // TODO: We should instead find the run ids and attempts that need to be updated.
            let first_message = chunk.first().ok_or_else(|| miette!("No first message"))?;

            let workflow_state = self
                .state
                .workflow_state(first_message.run_id, first_message.attempt);
            // TODO: Handle inputs better here.
            let context = OrchestrationContext::new(&workflow_state, self.inputs.clone());

            let actions = self
                .orchestrator
                .build_actions(context, Arc::clone(self.workflow_graph.as_ref().unwrap()))
                .await?;

            self.orchestrator.perform_actions(actions).await?;
        }

        Ok(())
    }

    async fn outputs(
        &mut self,
        run_id: Uuid,
        attempt: u32,
    ) -> miette::Result<HashMap<String, Vec<u8>>> {
        let workflow_state = self.state.workflow_state(run_id, attempt);

        let output_state = workflow_state
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
    let mut runtime = Runtime::memory()?;
    let (run_id, attempt) = runtime.start(workflow_graph, inputs).await?;
    runtime.run().await?;
    let outputs = runtime.outputs(run_id, attempt).await?;

    Ok(outputs)
}

/// Run a Workflow in memory from a filepath to a python file containing a builder.
///
/// # Errors
///
/// Will return Err if various I/O issues occur, if the runtime is unusable or
/// if the provided filepath includes invalid python.
pub fn run(path: &Path) -> miette::Result<()> {
    Python::attach(|py| {
        let path = path.canonicalize().into_diagnostic()?;

        let mut file = File::open(&path).into_diagnostic()?;
        let mut code_buf = String::new();
        file.read_to_string(&mut code_buf).into_diagnostic()?;
        code_buf.push('\x00');

        let code = CStr::from_bytes_until_nul(code_buf.as_bytes()).into_diagnostic()?;

        let source_file_name = path.file_name().ok_or_else(|| miette!("no file name"))?;
        let source_file_name_str = source_file_name
            .to_str()
            .ok_or_else(|| miette!("failed to convert to cstring"))?;
        let source_file_name_cstring = CString::new(source_file_name_str).into_diagnostic()?;

        let module = path.file_stem().ok_or_else(|| miette!("no file stem"))?;
        let module_name_cstring = CString::new(
            module
                .to_str()
                .ok_or_else(|| miette!("failed to convert to cstring"))?,
        )
        .into_diagnostic()?;

        let module = PyModule::from_code(py, code, &source_file_name_cstring, &module_name_cstring)
            .map_err(|err: PyErr| {
                if err.is_instance_of::<PySyntaxError>(py) {
                    let err_value = err.value(py);
                    let message: String = getattr_or_early_return!(err_value, "msg");
                    let lineno: usize = getattr_or_early_return!(err_value, "lineno");
                    let offset: usize = getattr_or_early_return!(err_value, "offset");
                    let end_offset: usize = getattr_or_early_return!(err_value, "end_offset");
                    let error_filename: String = getattr_or_early_return!(err_value, "filename");

                    let labels = vec![LabeledSpan::new_primary_with_span(
                        Some(message.clone()),
                        SourceSpan::new(
                            SourceOffset::from_location(&code_buf, lineno, offset),
                            end_offset - offset,
                        ),
                    )];
                    let diagnostic = MietteDiagnostic::new(message).and_labels(labels).with_help(
                        "Tierkreis requires a valid python module to construct a Workflow",
                    );

                    let source_code =
                        NamedSource::new(error_filename, code_buf.clone()).with_language("Python");

                    return Error::new(diagnostic).with_source_code(source_code);
                }

                miette!("Failed to load python module: {}", err.to_string())
            })?;

        let legacy_workflow_attr = module
            .getattr("workflow")
            .into_diagnostic()
            .map_err(|err| {
                let diagnostic = MietteDiagnostic::new("No 'workflow' attribute found in module")
                    .with_help("Tierkreis requires an attribute called 'workflow'");
                let rich_error = Error::new(diagnostic).with_source_code(
                    NamedSource::new(source_file_name_str, code_buf.clone())
                        .with_language("Python"),
                );
                rich_error.wrap_err(err)
            })?;

        let workflow_dump: String = legacy_workflow_attr
            .call_method0("model_dump_json")
            .into_diagnostic()?
            .to_string();

        let legacy_workflow: LegacyWorkflowGraph =
            serde_json::from_str(&workflow_dump).into_diagnostic()?;
        let workflow_graph = legacy_workflow.to_workflow_graph()?;

        // TODO: We cannot yet provide inputs to the workflow with this function.
        let outputs = run_workflow_in_memory(workflow_graph, HashMap::new())?;

        let outputs: HashMap<String, serde_json::Value> = outputs
            .into_iter()
            .map(|(k, v)| Ok((k.clone(), serde_json::from_slice(&v).into_diagnostic()?)))
            .collect::<miette::Result<_>>()?;

        println!("{outputs:?}");

        Ok(())
    })
}
