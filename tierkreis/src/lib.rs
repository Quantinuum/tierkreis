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
    use std::{collections::HashMap, sync::Arc};

    use miette::{Diagnostic, IntoDiagnostic};
    use num_complex::Complex64;
    use pyo3::{
        Borrowed, FromPyObject, PyErr, PyResult, Python, exceptions::PyValueError, prelude::*,
        types::PyBytes,
    };
    use pyo3_async_runtimes::tokio::get_runtime;
    use pythonize::depythonize;
    use serde::{Deserialize, Serialize};
    use tokio::{runtime::Builder, sync::oneshot};
    use tracing::info;
    use uuid::Uuid;

    use crate::{
        graph::{LegacyWorkflowGraph, WorkflowGraph},
        runtime::{self, RuntimeConfig},
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
    async fn new_from_config(config: PyRuntimeConfig) -> PyResult<PyRuntime> {
        let res = get_runtime()
            .spawn(async move { crate::runtime::Runtime::from_config(&config.0).await })
            .await
            .unwrap();

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
                .unwrap();

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
                .unwrap();

            match res {
                Ok((id, _attempt)) => Ok(id),
                Err(err) => Python::attach(|py| Err(convert_err(py, err))),
            }
        }

        async fn wait_for(&self, workflow_id: Uuid, attempt: u32) -> PyResult<()> {
            let inner = self.inner.clone();
            get_runtime()
                .spawn(async move {
                    let mut state_recv = inner.listen();
                    loop {
                        {
                            let updated = state_recv.borrow_and_update();
                            if !updated.active_runs.contains(&(workflow_id, attempt)) {
                                return Ok::<_, miette::Report>(());
                            }
                        }

                        state_recv.changed().await.into_diagnostic()?;
                    }
                })
                .await
                .unwrap()
                .unwrap();

            Ok(())
        }

        async fn get_outputs(&self, run_id: Uuid, attempt: u32) -> PyResult<ValueOrMapping> {
            let inner = self.inner.clone();
            let res = get_runtime()
                .spawn(async move { inner.get_outputs(run_id, attempt).await })
                .await
                .unwrap();

            match res {
                Ok(outputs) => Python::attach(|py| convert_outputs(py, outputs)),
                Err(err) => Python::attach(|py| Err(convert_err(py, err))),
            }
        }

        fn __enter__(&mut self) -> PyResult<()> {
            // We are already running a background runtime. Do nothing.
            if self.cancel.is_some() {
                return Ok(());
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
                })
            });

            self.cancel = Some(send);

            Ok(())
        }

        fn __exit__(
            &mut self,
            _exc_type: &Bound<'_, PyAny>,
            _exc_value: &Bound<'_, PyAny>,
            _traceback: &Bound<'_, PyAny>,
        ) {
            if let Some(cancel) = self.cancel.take() {
                cancel.send(()).unwrap();
            }
        }
    }

    impl Drop for PyRuntime {
        fn drop(&mut self) {
            if let Some(cancel) = self.cancel.take() {
                cancel.send(()).unwrap();
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

    #[pyfunction]
    fn run_workflow(
        py: Python<'_>,
        name: &str,
        workflow: PyWorkflowGraph,
        inputs: PyWorkflowInputs,
    ) -> PyResult<ValueOrMapping> {
        info!("starting workflow: '{name}'");
        let outputs = runtime::run_workflow_in_memory(workflow.0, inputs.0)
            .map_err(|err| convert_err(py, err))?;

        info!("workflow finished: '{name}'");
        convert_outputs(py, outputs)
    }
}
