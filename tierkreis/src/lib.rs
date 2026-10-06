/*! This is the library module for the rust components of the Tierkreis
Workflow Management system.
*/
#![warn(missing_docs)]
#![warn(clippy::all, clippy::pedantic)]

pub mod asset_storage;
#[cfg(test)]
pub mod builder;
pub mod config;
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
    use std::{collections::HashMap, sync::Arc, time::Duration};

    use chrono::{TimeDelta, Utc};
    use miette::{Diagnostic, IntoDiagnostic, miette};
    use num_complex::Complex64;
    use pyo3::{
        Borrowed, FromPyObject, PyErr, PyResult, Python, exceptions::PyValueError, prelude::*,
        types::PyBytes,
    };
    use pyo3_async_runtimes::tokio::get_runtime;
    use pythonize::depythonize;
    use serde::{Deserialize, Serialize};
    use tokio::{runtime::Builder, sync::oneshot};
    use tracing::warn;
    use uuid::Uuid;

    use crate::{
        graph::{LegacyWorkflowGraph, WorkflowGraph},
        location::Location,
        runtime::RuntimeConfig,
    };

    #[allow(clippy::unnecessary_wraps)]
    #[pymodule_init]
    fn init(_m: &Bound<'_, PyModule>) -> PyResult<()> {
        let mut builder = Builder::new_multi_thread();
        builder.enable_all();
        pyo3_async_runtimes::tokio::init(builder);
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
            py_err
                .add_note(py, format!("\thelp: {help}"))
                .expect("Failed to add help annotation to exception");
        }
        if let Some(related) = err.related() {
            for related in related {
                py_err
                    .add_note(py, format!("related: {related}"))
                    .expect("Failed to add related note to exception");
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

    #[derive(Debug, FromPyObject, IntoPyObject, Serialize, Deserialize)]
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

    #[pyclass(name = "NodeState")]
    #[derive(Debug)]
    struct PyNodeState {
        #[pyo3(get)]
        pub status: String,
    }

    #[pyclass(name = "WorkflowSummary")]
    #[derive(Debug)]
    pub struct PyWorkflowRunStateSummary {
        #[pyo3(get)]
        pub name: Option<String>,
        #[pyo3(get)]
        pub duration: Option<TimeDelta>,
        /// The time that the workflow started.
        #[pyo3(get)]
        pub started_time: Option<chrono::DateTime<Utc>>,
        /// The time that the workflow was queued.
        #[pyo3(get)]
        pub queued_time: Option<chrono::DateTime<Utc>>,
        /// The time that the workflow completed.
        #[pyo3(get)]
        pub complete_time: Option<chrono::DateTime<Utc>>,
        /// The time that the workflow was cancelled.
        #[pyo3(get)]
        pub cancelled_time: Option<chrono::DateTime<Utc>>,
        /// The time that the workflow errored.
        #[pyo3(get)]
        pub error_time: Option<chrono::DateTime<Utc>>,
        /// Locations of nodes that have errored in this run.
        #[pyo3(get)]
        pub errored_locations: Vec<String>,
    }

    #[pyclass]
    enum NodeStatus {
        Complete,
        Cancelled,
        Error,
        Running,
        Queued,
        Scheduled,
    }

    impl From<crate::state::interface::NodeState> for PyNodeState {
        fn from(node_state: crate::state::interface::NodeState) -> Self {
            let status = if node_state.complete_time.is_some() {
                "Complete".to_string()
            } else if node_state.cancelled_time.is_some() {
                "Cancelled".to_string()
            } else if node_state.error_time.is_some() {
                "Error".to_string()
            } else if node_state.running_time.is_some() {
                "Running".to_string()
            } else if node_state.queued_time.is_some() {
                "Queued".to_string()
            } else if node_state.scheduled_time.is_some() {
                "Scheduled".to_string()
            } else {
                "Unknown".to_string()
            };
            Self { status }
        }
    }

    #[pyclass(name = "Runtime")]
    struct PyRuntime {
        inner: Arc<crate::runtime::Runtime>,
        cancel: Option<oneshot::Sender<()>>,
    }

    #[pyfunction]
    async fn new_default() -> PyResult<PyRuntime> {
        new_from_config(PyRuntimeConfig(RuntimeConfig::default())).await
    }

    #[pyfunction]
    async fn new_in_memory() -> PyResult<PyRuntime> {
        new_from_config(PyRuntimeConfig(RuntimeConfig::memory())).await
    }

    #[pyfunction]
    async fn new_sqlite_memory() -> PyResult<PyRuntime> {
        new_from_config(PyRuntimeConfig(RuntimeConfig::sqlite_memory())).await
    }

    #[pyfunction]
    async fn new_from_config(config: PyRuntimeConfig) -> PyResult<PyRuntime> {
        let res = get_runtime()
            .spawn(async move { crate::runtime::Runtime::from_config(&config.0).await })
            .await
            .map_err(|err| {
                Python::attach(|py| convert_err(py, miette!("Failed to join future: {err}")))
            })?;

        match res {
            Ok(runtime) => Ok(PyRuntime {
                inner: Arc::new(runtime),
                cancel: None,
            }),
            Err(err) => Python::attach(|py| Err(convert_err(py, err))),
        }
    }

    #[pymethods]
    impl PyRuntime {
        async fn save_workflow(
            &self,
            name: Option<String>,
            workflow: PyWorkflowGraph,
        ) -> PyResult<Uuid> {
            let inner = self.inner.clone();
            let res = get_runtime()
                .spawn(async move { inner.save_workflow(name, workflow.0).await })
                .await
                .map_err(|err| {
                    Python::attach(|py| convert_err(py, miette!("Failed to join future: {err}")))
                })?;

            match res {
                Ok(id) => Ok(id),
                Err(err) => Python::attach(|py| Err(convert_err(py, err))),
            }
        }

        async fn start_new_run(
            &self,
            workflow_id: Uuid,
            inputs: PyWorkflowInputs,
        ) -> PyResult<Uuid> {
            let inner = self.inner.clone();
            let res = get_runtime()
                .spawn(async move { inner.start_new_run(workflow_id, inputs.0).await })
                .await
                .map_err(|err| {
                    Python::attach(|py| convert_err(py, miette!("Failed to join future: {err}")))
                })?;

            match res {
                Ok((id, _attempt)) => Ok(id),
                Err(err) => Python::attach(|py| Err(convert_err(py, err))),
            }
        }

        #[pyo3(signature = (run_id, attempt=0, /, timeout=604_800))]
        async fn wait_for(&self, run_id: Uuid, attempt: u32, timeout: u64) -> PyResult<()> {
            let inner = self.inner.clone();
            get_runtime()
                .spawn(async move {
                    let fut = inner.wait_for(run_id, attempt);
                    tokio::time::timeout(Duration::from_secs(timeout), fut)
                        .await
                        .into_diagnostic()??;
                    Ok(())
                })
                .await
                .map_err(|err| {
                    Python::attach(|py| convert_err(py, miette!("Failed to join future: {err}")))
                })?
                .map_err(|err| Python::attach(|py| convert_err(py, err)))?;

            Ok(())
        }

        #[pyo3(signature = (run_id, attempt=0, /))]
        async fn get_summary(
            &self,
            run_id: Uuid,
            attempt: u32,
        ) -> PyResult<PyWorkflowRunStateSummary> {
            let inner = self.inner.clone();
            let res = get_runtime()
                .spawn(async move { inner.read_workflow_run_summary(run_id, attempt).await })
                .await
                .map_err(|err| {
                    Python::attach(|py| convert_err(py, miette!("Failed to join future: {err}")))
                })?;

            match res {
                Ok(summary) => {
                    let duration = summary.started_time.and_then(|started_time| {
                        let finished_time = summary
                            .complete_time
                            .or(summary.error_time)
                            .or(summary.cancelled_time);
                        finished_time.map(|finished_time| finished_time - started_time)
                    });
                    Ok(PyWorkflowRunStateSummary {
                        name: summary.name,
                        duration,
                        started_time: summary.started_time,
                        queued_time: summary.queued_time,
                        complete_time: summary.complete_time,
                        cancelled_time: summary.cancelled_time,
                        error_time: summary.error_time,
                        errored_locations: summary
                            .errored_locations
                            .into_iter()
                            .map(|loc| loc.to_string())
                            .collect(),
                    })
                }
                Err(err) => Python::attach(|py| Err(convert_err(py, err))),
            }
        }

        #[pyo3(signature = (run_id, attempt=0, /))]
        async fn get_outputs(&self, run_id: Uuid, attempt: u32) -> PyResult<ValueOrMapping> {
            let inner = self.inner.clone();
            let res = get_runtime()
                .spawn(async move { inner.get_outputs(run_id, attempt).await })
                .await
                .map_err(|err| {
                    Python::attach(|py| convert_err(py, miette!("Failed to join future: {err}")))
                })?;

            match res {
                Ok(outputs) => Python::attach(|py| convert_outputs(py, outputs)),
                Err(err) => Python::attach(|py| Err(convert_err(py, err))),
            }
        }

        async fn debug_read_node_states(
            &self,
            run_id: Uuid,
            attempt: u32,
            locations: Vec<String>,
        ) -> PyResult<HashMap<String, PyNodeState>> {
            let inner = self.inner.clone();
            let locations = locations
                .iter()
                .map(|s| Location::new(s))
                .collect::<Result<Vec<_>, _>>()
                .map_err(|err| Python::attach(|py| convert_err(py, err)))?;

            let res = get_runtime()
                .spawn(async move {
                    inner
                        .read_node_states(run_id, attempt, locations.into_iter())
                        .await
                })
                .await
                .map_err(|err| {
                    Python::attach(|py| convert_err(py, miette!("Failed to join future: {err}")))
                })?;

            match res {
                Ok(states) => Ok(states
                    .into_iter()
                    .map(|(k, v)| (k.to_string(), v.into()))
                    .collect()),
                Err(err) => Python::attach(|py| Err(convert_err(py, err))),
            }
        }

        #[pyo3(signature = (run_id, locations=None, attempt=0))]
        async fn restart_task(
            &self,
            run_id: Uuid,
            locations: Option<Vec<String>>,
            attempt: u32,
        ) -> PyResult<(u32, Vec<String>)> {
            let locations = locations.unwrap_or_default();
            let inner = self.inner.clone();
            let locations = locations
                .iter()
                .map(|s| Location::new(s))
                .collect::<Result<Vec<_>, _>>()
                .map_err(|err| Python::attach(|py| convert_err(py, err)))?;

            let res = get_runtime()
                .spawn(async move { inner.restart_task(run_id, attempt, locations).await })
                .await
                .map_err(|err| {
                    Python::attach(|py| convert_err(py, miette!("Failed to join future: {err}")))
                })?;

            match res {
                Ok((new_attempt, invalidated)) => Ok((
                    new_attempt,
                    invalidated.into_iter().map(|loc| loc.to_string()).collect(),
                )),
                Err(err) => Python::attach(|py| Err(convert_err(py, err))),
            }
        }

        fn __enter__(&mut self) {
            // We are already running a background runtime. Do nothing.
            if self.cancel.is_some() {
                return;
            }

            let (send, recv) = oneshot::channel::<()>();
            let runtime = self.inner.clone();
            let rt = pyo3_async_runtimes::tokio::get_runtime();
            std::thread::spawn(move || {
                rt.block_on(async move {
                    tokio::select! {
                        _ = recv => {}
                        Err(err) = runtime.run() => {
                            tracing::error!("Error while processing events: {err}");
                            eprintln!("{err}");
                        }
                        else => {},
                    }
                });
            });

            self.cancel = Some(send);
        }

        fn __exit__(
            &mut self,
            _exc_type: &Bound<'_, PyAny>,
            _exc_value: &Bound<'_, PyAny>,
            _traceback: &Bound<'_, PyAny>,
        ) {
            if let Some(cancel) = self.cancel.take()
                && cancel.send(()).is_err()
            {
                warn!("Failed to cancel background Runtime thread, it may have already terminated");
            }
        }
    }

    impl Drop for PyRuntime {
        fn drop(&mut self) {
            if let Some(cancel) = self.cancel.take()
                && cancel.send(()).is_err()
            {
                warn!("Failed to cancel background Runtime thread, it may have already terminated");
            }
        }
    }

    // TODO: I'm not sure how debuggable this is
    #[pyclass(name = "RuntimeConfig")]
    struct PyRuntimeConfig(pub RuntimeConfig);

    impl<'a, 'py> FromPyObject<'a, 'py> for PyRuntimeConfig {
        type Error = PyErr;

        fn extract(obj: Borrowed<'a, 'py, PyAny>) -> PyResult<Self> {
            let config = depythonize(&obj)?;

            Ok(PyRuntimeConfig(config))
        }
    }

    // TODO: I'm not sure how debuggable this is
    #[pyclass(name = "WorkflowGraph")]
    struct PyWorkflowGraph(pub WorkflowGraph);

    impl<'a, 'py> FromPyObject<'a, 'py> for PyWorkflowGraph {
        type Error = PyErr;

        fn extract(obj: Borrowed<'a, 'py, PyAny>) -> PyResult<Self> {
            let py = obj.py();
            let workflow_dump: String = if obj.hasattr("data")? {
                let graph = obj.getattr("data")?;
                graph
                    .call_method0("model_dump_json")
                    .into_diagnostic()
                    .map_err(|err| convert_err(py, err))?
                    .to_string()
            } else {
                obj.call_method0("model_dump_json")
                    .into_diagnostic()
                    .map_err(|err| convert_err(py, err))?
                    .to_string()
            };

            let legacy_workflow: LegacyWorkflowGraph = serde_json::from_str(&workflow_dump)
                .into_diagnostic()
                .map_err(|err| convert_err(py, err))?;
            let workflow_graph = legacy_workflow
                .to_workflow_graph()
                .map_err(|err| convert_err(py, err))?;

            Ok(PyWorkflowGraph(workflow_graph))
        }
    }

    // TODO: I'm not sure how debuggable this is
    #[pyclass]
    struct PyWorkflowInputs(pub HashMap<String, Vec<u8>>);

    impl<'a, 'py> FromPyObject<'a, 'py> for PyWorkflowInputs {
        type Error = PyErr;

        fn extract(obj: Borrowed<'a, 'py, PyAny>) -> PyResult<Self> {
            let py = obj.py();
            Ok(Self(match obj.extract()? {
                ValueOrMappingOrBytes::BytesMapping(bytes_mapping) => bytes_mapping
                    .into_iter()
                    .map(|(k, v)| Ok::<_, miette::Report>((k, v.extract().into_diagnostic()?)))
                    .collect::<Result<HashMap<String, Vec<u8>>, _>>()
                    .map_err(|err| convert_err(py, err))?,
                ValueOrMappingOrBytes::ValueOrBytes(ValueOrMapping::Value(value)) => {
                    let mut inputs = HashMap::new();
                    inputs.insert(
                        "value".to_string(),
                        serde_json::to_vec(&value)
                            .into_diagnostic()
                            .map_err(|err| convert_err(py, err))?,
                    );
                    inputs
                }
                ValueOrMappingOrBytes::ValueOrBytes(ValueOrMapping::Mapping(inputs)) => inputs
                    .into_iter()
                    .map(|(k, v)| serde_json::to_vec(&v).map(|b| (k, b)))
                    .collect::<Result<HashMap<_, _>, _>>()
                    .into_diagnostic()
                    .map_err(|err| convert_err(py, err))?,
            }))
        }
    }

    fn convert_outputs(
        py: Python<'_>,
        outputs: HashMap<String, Vec<u8>>,
    ) -> PyResult<ValueOrMapping> {
        let mut outputs: HashMap<String, Option<Value>> = outputs
            .into_iter()
            .map(|(k, v)| Ok((k.clone(), serde_json::from_slice(&v).into_diagnostic()?)))
            .collect::<miette::Result<_>>()
            .map_err(|err| convert_err(py, err))?;

        if outputs.len() == 1 && outputs.contains_key("value") {
            Ok(ValueOrMapping::Value(
                outputs
                    .remove("value")
                    .flatten()
                    .expect("No single output value"),
            ))
        } else {
            Ok(ValueOrMapping::Mapping(outputs))
        }
    }
}
