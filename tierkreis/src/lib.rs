/*! This is the library module for the rust components of the Tierkreis
Workflow Management system.
*/
#![warn(missing_docs)]
#![warn(clippy::all, clippy::pedantic)]

pub mod asset_storage;
#[cfg(test)]
pub mod builder;
pub mod event;
pub mod executor;
pub mod graph;
pub mod location;
pub mod monitoring;
pub mod orchestrator;
pub mod runtime;
pub mod server;
pub mod state;

#[pyo3::pymodule]
#[pyo3(name = "_tierkreis")]
mod tierkreis {
    use std::{collections::HashMap, path::PathBuf, sync::Arc};

    use miette::{Diagnostic, IntoDiagnostic, miette};
    use num_complex::Complex64;
    use pyo3::{FromPyObject, PyErr, Python, exceptions::PyValueError, prelude::*, types::PyBytes};
    use serde::{Deserialize, Serialize};
    use tracing::info;
    use uuid::Uuid;

    use crate::{
        asset_storage::{AssetStorageRegistry, load_assets, save_assets},
        graph::{LegacyWorkflowGraph, WorkflowGraph},
        runtime::{self, Runtime as NativeRuntime, RuntimeConfig},
        state::{RuntimeState, interface::NodeState},
    };

    #[allow(clippy::unnecessary_wraps)]
    #[pymodule_init]
    fn init(_m: &Bound<'_, PyModule>) -> PyResult<()> {
        Ok(())
    }

    fn convert_err(py: Python<'_>, err: miette::Report) -> PyErr {
        let boxed: Box<dyn Diagnostic + 'static> = err.into();
        convert_diagnostic(py, &*boxed)
    }

    fn convert_diagnostic(py: Python<'_>, err: &dyn Diagnostic) -> PyErr {
        let py_err = PyValueError::new_err(err.to_string());

        if let Some(source) = err.diagnostic_source() {
            py_err.set_cause(py, Some(convert_diagnostic(py, source)));
        } else if let Some(source) = err.source() {
            py_err.set_cause(py, Some(convert_stderr(py, source)));
        }
        if let Some(help) = err.help() {
            py_err.add_note(py, format!("\thelp: {help}")).unwrap();
        }
        if let Some(related) = err.related() {
            for related in related {
                py_err.add_note(py, format!("related: {related}")).unwrap();
            }
        }
        py_err
    }

    fn convert_stderr(py: Python<'_>, err: &dyn std::error::Error) -> PyErr {
        let py_err = PyValueError::new_err(err.to_string());
        if let Some(source) = err.source() {
            py_err.set_cause(py, Some(convert_stderr(py, source)));
        }
        py_err
    }

    #[derive(Debug, FromPyObject, IntoPyObject)]
    enum ValueOrMappingOrBytes<'py> {
        #[pyo3(transparent, annotation = "dict[str, bytes]")]
        BytesMapping(HashMap<String, Bound<'py, PyBytes>>),
        #[pyo3(transparent)]
        ValueOrBytes(ValueOrMapping),
    }

    #[derive(Debug, FromPyObject, IntoPyObject)]
    enum ValueOrMapping {
        #[pyo3(transparent, annotation = "Mapping")]
        Mapping(HashMap<String, Option<Value>>),
        #[pyo3(transparent)]
        Value(Value),
    }

    #[derive(Clone, Debug, FromPyObject, IntoPyObject, Serialize, Deserialize)]
    #[serde(untagged)]
    enum Value {
        #[pyo3(transparent, annotation = "bool")]
        Bool(bool),
        #[pyo3(transparent, annotation = "int")]
        Int(i64),
        #[pyo3(transparent, annotation = "float")]
        Float(f64),
        #[pyo3(transparent, annotation = "str")]
        String(String),
        #[pyo3(transparent, annotation = "complex")]
        Complex(Complex64),
        #[pyo3(transparent, annotation = "Sequence")]
        List(Vec<Value>),
        #[pyo3(transparent, annotation = "Mapping")]
        Dict(HashMap<String, Value>),
    }

    fn workflow_from_python(
        py: Python<'_>,
        workflow: &Bound<'_, PyAny>,
    ) -> PyResult<WorkflowGraph> {
        let graph = workflow
            .getattr("data")
            .unwrap_or_else(|_| workflow.clone());
        let workflow_dump: String = graph
            .call_method0("model_dump_json")
            .into_diagnostic()
            .map_err(|err| convert_err(py, err))?
            .to_string();
        let legacy_workflow: LegacyWorkflowGraph = serde_json::from_str(&workflow_dump)
            .into_diagnostic()
            .map_err(|err| convert_err(py, err))?;
        legacy_workflow
            .to_workflow_graph()
            .map_err(|err| convert_err(py, err))
    }

    fn inputs_from_python(
        py: Python<'_>,
        inputs: ValueOrMappingOrBytes<'_>,
    ) -> PyResult<HashMap<String, Vec<u8>>> {
        match inputs {
            ValueOrMappingOrBytes::BytesMapping(bytes_mapping) => bytes_mapping
                .into_iter()
                .map(|(k, v)| Ok::<_, miette::Report>((k, v.extract().into_diagnostic()?)))
                .collect::<Result<HashMap<String, Vec<u8>>, _>>()
                .map_err(|err| convert_err(py, err)),
            ValueOrMappingOrBytes::ValueOrBytes(ValueOrMapping::Value(value)) => {
                Ok(HashMap::from([(
                    "value".to_string(),
                    serde_json::to_vec(&value)
                        .into_diagnostic()
                        .map_err(|err| convert_err(py, err))?,
                )]))
            }
            ValueOrMappingOrBytes::ValueOrBytes(ValueOrMapping::Mapping(inputs)) => inputs
                .into_iter()
                .map(|(k, v)| serde_json::to_vec(&v).map(|b| (k, b)))
                .collect::<Result<HashMap<_, _>, _>>()
                .into_diagnostic()
                .map_err(|err| convert_err(py, err)),
        }
    }

    fn outputs_to_python(
        py: Python<'_>,
        outputs: HashMap<String, Vec<u8>>,
    ) -> PyResult<ValueOrMapping> {
        let mut outputs: HashMap<String, Option<Value>> = outputs
            .into_iter()
            .map(|(k, v)| Ok((k, serde_json::from_slice(&v).into_diagnostic()?)))
            .collect::<miette::Result<_>>()
            .map_err(|err| convert_err(py, err))?;
        if outputs.len() == 1 && outputs.contains_key("value") {
            Ok(ValueOrMapping::Value(
                outputs
                    .remove("value")
                    .flatten()
                    .ok_or_else(|| PyValueError::new_err("Single output `value` was null"))?,
            ))
        } else {
            Ok(ValueOrMapping::Mapping(outputs))
        }
    }

    fn node_status(state: &NodeState) -> &'static str {
        if state.complete_time.is_some() {
            "Completed"
        } else if state.error_time.is_some() {
            "Errored"
        } else if state.cancelled_time.is_some() {
            "Cancelled"
        } else if state.running_time.is_some() {
            "Running"
        } else if state.queued_time.is_some() {
            "Queued"
        } else if state.scheduled_time.is_some() {
            "Scheduled"
        } else {
            "NotStarted"
        }
    }

    #[pyclass(frozen, get_all, skip_from_py_object)]
    #[derive(Clone)]
    struct NodeStateSnapshot {
        location: String,
        status: String,
        scheduled_time: Option<String>,
        queued_time: Option<String>,
        running_time: Option<String>,
        complete_time: Option<String>,
        cancelled_time: Option<String>,
        error_time: Option<String>,
        outputs: HashMap<String, Option<Value>>,
        condition: Option<bool>,
        loop_index: Option<u32>,
        map_completed: Option<Vec<bool>>,
        error: Option<String>,
        error_detail: Option<String>,
        task_handle: Option<String>,
    }

    #[pyclass(frozen, get_all)]
    struct WorkflowStateSnapshot {
        workflow_id: Uuid,
        run_id: Uuid,
        attempt: u32,
        name: Option<String>,
        status: String,
        started_time: Option<String>,
        metadata: HashMap<String, String>,
        nodes: HashMap<String, NodeStateSnapshot>,
    }

    #[pyclass(name = "Runtime")]
    struct PyRuntime {
        tokio: tokio::runtime::Runtime,
        state: Arc<dyn RuntimeState>,
        assets: AssetStorageRegistry,
        default_storage_name: String,
        engine: Option<std::thread::JoinHandle<()>>,
        shutdown: tokio::sync::watch::Sender<bool>,
    }

    impl PyRuntime {
        fn from_config(py: Python<'_>, config: RuntimeConfig) -> PyResult<Self> {
            let (ready_sender, ready_receiver) = std::sync::mpsc::sync_channel(1);
            let (shutdown, shutdown_receiver) = tokio::sync::watch::channel(false);
            let engine = std::thread::spawn(move || {
                let tokio = tokio::runtime::Builder::new_multi_thread()
                    .enable_all()
                    .build()
                    .expect("failed to create runtime engine");
                tokio.block_on(async move {
                    match NativeRuntime::from_config(&config).await {
                        Ok(mut runtime) => {
                            let ready = (
                                runtime.state(),
                                runtime.asset_storage_registry(),
                                runtime.default_storage_name(),
                            );
                            if ready_sender.send(Ok(ready)).is_err() {
                                return;
                            }
                            runtime.set_shutdown(shutdown_receiver);
                            if let Err(error) = runtime.run().await {
                                tracing::error!("Python runtime engine stopped: {error}");
                            }
                        }
                        Err(error) => {
                            let _ = ready_sender.send(Err(error.to_string()));
                        }
                    }
                });
            });
            let (state, assets, default_storage_name) = ready_receiver
                .recv()
                .into_diagnostic()
                .map_err(|err| convert_err(py, err))?
                .map_err(PyValueError::new_err)?;
            let tokio = tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
                .into_diagnostic()
                .map_err(|err| convert_err(py, err))?;
            Ok(Self {
                tokio,
                state,
                assets,
                default_storage_name,
                engine: Some(engine),
                shutdown,
            })
        }

        async fn summary(
            state: Arc<dyn RuntimeState>,
            run_id: Uuid,
            attempt: Option<u32>,
        ) -> miette::Result<crate::state::queries::WorkflowRunSummary> {
            let mut summaries = state
                .list_workflow_run_summaries()
                .await?
                .into_iter()
                .filter(|summary| summary.run_id == run_id)
                .collect::<Vec<_>>();
            summaries.sort_by_key(|summary| summary.attempt);
            match attempt {
                Some(attempt) => summaries
                    .into_iter()
                    .find(|summary| summary.attempt == attempt),
                None => summaries.pop(),
            }
            .ok_or_else(|| miette!("Workflow run not found: {run_id}"))
        }

        async fn raw_outputs(
            state: Arc<dyn RuntimeState>,
            assets: AssetStorageRegistry,
            run_id: Uuid,
            attempt: u32,
        ) -> miette::Result<HashMap<String, Vec<u8>>> {
            let run_state = state.load_workflow_run_state(run_id, attempt).await?;
            let workflow = state.load_workflow(run_state.workflow_id()).await?;
            let output_state = run_state
                .read(&crate::location::Location::from_node_index_iter([
                    workflow.output_idx()
                ]))
                .await?;
            load_assets(
                &assets,
                &output_state
                    .outputs
                    .ok_or_else(|| miette!("Workflow run has no outputs"))?,
            )
            .await
        }
    }

    #[pymethods]
    impl PyRuntime {
        #[new]
        fn new(py: Python<'_>) -> PyResult<Self> {
            Self::from_config(py, RuntimeConfig::memory())
        }

        #[staticmethod]
        #[pyo3(signature = (database_path, asset_dir, *, executor = "memory"))]
        fn sqlite(
            py: Python<'_>,
            database_path: PathBuf,
            asset_dir: PathBuf,
            executor: &str,
        ) -> PyResult<Self> {
            if !matches!(executor, "memory" | "subprocess") {
                return Err(PyValueError::new_err(
                    "executor must be 'memory' or 'subprocess'",
                ));
            }
            std::fs::create_dir_all(&asset_dir)
                .into_diagnostic()
                .map_err(|err| convert_err(py, err))?;
            Self::from_config(
                py,
                RuntimeConfig::sqlite(
                    database_path
                        .into_os_string()
                        .to_string_lossy()
                        .into_owned(),
                    asset_dir,
                    executor == "subprocess",
                ),
            )
        }

        fn upload_workflow(
            &self,
            py: Python<'_>,
            name: String,
            workflow: &Bound<'_, PyAny>,
        ) -> PyResult<Uuid> {
            let workflow = workflow_from_python(py, workflow)?;
            self.tokio
                .block_on(self.state.save_workflow(Some(name), workflow))
                .map_err(|err| convert_err(py, err))
        }

        fn start_workflow(
            &self,
            py: Python<'_>,
            workflow_id: Uuid,
            inputs: ValueOrMappingOrBytes<'_>,
        ) -> PyResult<Uuid> {
            let inputs = inputs_from_python(py, inputs)?;
            let assets = Arc::clone(&self.assets);
            let storage_name = self.default_storage_name.clone();
            let state = Arc::clone(&self.state);
            self.tokio
                .block_on(async move {
                    state.load_workflow(workflow_id).await?;
                    let inputs = save_assets(&assets, &storage_name, inputs).await?;
                    Ok::<_, miette::Report>(
                        state
                            .new_workflow_run_state(workflow_id, inputs)
                            .await?
                            .run_id(),
                    )
                })
                .map_err(|err| convert_err(py, err))
        }

        #[pyo3(signature = (run_id, *, attempt = None, timeout = None))]
        fn wait(
            &self,
            py: Python<'_>,
            run_id: Uuid,
            attempt: Option<u32>,
            timeout: Option<f64>,
        ) -> PyResult<String> {
            let state = Arc::clone(&self.state);
            self.tokio
                .block_on(async move {
                    let summary = Self::summary(Arc::clone(&state), run_id, attempt).await?;
                    let attempt = summary.attempt;
                    let mut updates = state.listen();
                    let wait = async {
                        loop {
                            if !updates
                                .borrow_and_update()
                                .active_runs
                                .contains(&(run_id, attempt))
                            {
                                return Self::summary(Arc::clone(&state), run_id, Some(attempt))
                                    .await
                                    .map(|summary| {
                                        summary.status.unwrap_or_else(|| "Pending".to_string())
                                    });
                            }
                            updates.changed().await.into_diagnostic()?;
                        }
                    };
                    match timeout {
                        Some(seconds) => {
                            tokio::time::timeout(std::time::Duration::from_secs_f64(seconds), wait)
                                .await
                                .map_err(|_| {
                                    miette!("Timed out waiting for workflow run {run_id}")
                                })?
                        }
                        None => wait.await,
                    }
                })
                .map_err(|err| convert_err(py, err))
        }

        #[pyo3(signature = (run_id, *, attempt = None))]
        fn status(&self, py: Python<'_>, run_id: Uuid, attempt: Option<u32>) -> PyResult<String> {
            self.tokio
                .block_on(Self::summary(Arc::clone(&self.state), run_id, attempt))
                .map(|summary| summary.status.unwrap_or_else(|| "Pending".to_string()))
                .map_err(|err| convert_err(py, err))
        }

        #[pyo3(signature = (run_id, *, attempt = None))]
        fn get_outputs(
            &self,
            py: Python<'_>,
            run_id: Uuid,
            attempt: Option<u32>,
        ) -> PyResult<ValueOrMapping> {
            let summary = self
                .tokio
                .block_on(Self::summary(Arc::clone(&self.state), run_id, attempt))
                .map_err(|err| convert_err(py, err))?;
            match summary.status.as_deref() {
                Some("Completed") => {}
                Some(status @ ("Errored" | "Cancelled")) => {
                    return Err(PyValueError::new_err(format!(
                        "Workflow run {run_id} finished with status {status}"
                    )));
                }
                _ => {
                    return Err(PyValueError::new_err(format!(
                        "Workflow run {run_id} has not finished"
                    )));
                }
            }
            let outputs = self
                .tokio
                .block_on(Self::raw_outputs(
                    Arc::clone(&self.state),
                    Arc::clone(&self.assets),
                    run_id,
                    summary.attempt,
                ))
                .map_err(|err| convert_err(py, err))?;
            outputs_to_python(py, outputs)
        }

        #[pyo3(signature = (run_id, *, attempt = None))]
        fn get_workflow_state(
            &self,
            py: Python<'_>,
            run_id: Uuid,
            attempt: Option<u32>,
        ) -> PyResult<WorkflowStateSnapshot> {
            let state = Arc::clone(&self.state);
            let assets = Arc::clone(&self.assets);
            self.tokio
                .block_on(async move {
                    let summary = Self::summary(Arc::clone(&state), run_id, attempt).await?;
                    let run_state = state
                        .load_workflow_run_state(run_id, summary.attempt)
                        .await?;
                    let metadata = run_state.read_metadata().await?;
                    let mut nodes = HashMap::new();
                    for (location, node) in run_state.read_all().await? {
                        let raw_outputs = match node.outputs.as_ref() {
                            Some(outputs) => load_assets(&assets, outputs).await?,
                            None => HashMap::new(),
                        };
                        let outputs = raw_outputs
                            .into_iter()
                            .map(|(name, value)| {
                                Ok((name, serde_json::from_slice(&value).into_diagnostic()?))
                            })
                            .collect::<miette::Result<_>>()?;
                        let location = location.to_string();
                        nodes.insert(
                            location.clone(),
                            NodeStateSnapshot {
                                location,
                                status: node_status(&node).to_string(),
                                scheduled_time: node.scheduled_time.map(|time| time.to_rfc3339()),
                                queued_time: node.queued_time.map(|time| time.to_rfc3339()),
                                running_time: node.running_time.map(|time| time.to_rfc3339()),
                                complete_time: node.complete_time.map(|time| time.to_rfc3339()),
                                cancelled_time: node.cancelled_time.map(|time| time.to_rfc3339()),
                                error_time: node.error_time.map(|time| time.to_rfc3339()),
                                outputs,
                                condition: node.cond,
                                loop_index: node.loop_index,
                                map_completed: node
                                    .map_completed
                                    .map(|bits| bits.iter().by_vals().collect()),
                                error: node.error,
                                error_detail: node.error_detail,
                                task_handle: node.handle,
                            },
                        );
                    }
                    Ok::<_, miette::Report>(WorkflowStateSnapshot {
                        workflow_id: summary.workflow_id,
                        run_id,
                        attempt: summary.attempt,
                        name: summary.name,
                        status: summary.status.unwrap_or_else(|| "Pending".to_string()),
                        started_time: summary.started_time.map(|time| time.to_rfc3339()),
                        metadata,
                        nodes,
                    })
                })
                .map_err(|err| convert_err(py, err))
        }

        fn close(&mut self) {
            self.shutdown.send_replace(true);
            if let Some(engine) = self.engine.take() {
                let _ = engine.join();
            }
        }

        fn __enter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
            slf
        }

        fn __exit__(
            &mut self,
            _exc_type: &Bound<'_, PyAny>,
            _exc_value: &Bound<'_, PyAny>,
            _traceback: &Bound<'_, PyAny>,
        ) {
            self.close();
        }
    }

    impl Drop for PyRuntime {
        fn drop(&mut self) {
            self.close();
        }
    }

    #[pyfunction]
    fn run_workflow(
        py: Python<'_>,
        name: &str,
        workflow: &Bound<'_, PyAny>,
        inputs: ValueOrMappingOrBytes,
    ) -> PyResult<ValueOrMapping> {
        info!("starting workflow: '{name}'");
        let workflow_graph = workflow_from_python(py, workflow)?;
        let inputs = inputs_from_python(py, inputs)?;

        let outputs = runtime::run_workflow_in_memory(workflow_graph, inputs)
            .map_err(|err| convert_err(py, err))?;

        info!("done with outputs: {outputs:?}");
        outputs_to_python(py, outputs)
    }
}
