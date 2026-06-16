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
pub mod orchestrator;
pub mod runtime;
pub mod server;
pub mod state;
pub mod updater;

#[pyo3::pymodule]
#[pyo3(name = "_tierkreis")]
mod tierkreis {
    use std::collections::HashMap;

    use miette::IntoDiagnostic;
    use num_complex::Complex64;
    use pyo3::{FromPyObject, PyErr, Python, exceptions::PyValueError, prelude::*, types::PyBytes};
    use serde::{Deserialize, Serialize};
    use tracing::info;

    use crate::{graph::LegacyWorkflowGraph, runtime};

    #[allow(clippy::unnecessary_wraps)]
    #[pymodule_init]
    fn init(_m: &Bound<'_, PyModule>) -> PyResult<()> {
        // console_subscriber::init();
        tracing_subscriber::fmt().compact().init();
        Ok(())
    }

    fn convert_err(py: Python<'_>, err: &miette::Report) -> PyErr {
        let py_err = PyValueError::new_err(err.to_string());
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

    #[pyfunction]
    fn run_workflow(
        py: Python<'_>,
        name: &str,
        workflow: &Bound<'_, PyAny>,
        inputs: ValueOrMappingOrBytes,
    ) -> PyResult<ValueOrMapping> {
        info!("starting workflow: '{name}'");
        let workflow_dump: String = workflow
            .call_method0("model_dump_json")
            .into_diagnostic()
            .map_err(|err| convert_err(py, &err))?
            .to_string();

        let legacy_workflow: LegacyWorkflowGraph = serde_json::from_str(&workflow_dump)
            .into_diagnostic()
            .map_err(|err| convert_err(py, &err))?;
        let workflow_graph = legacy_workflow
            .to_workflow_graph()
            .map_err(|err| convert_err(py, &err))?;

        let inputs = match inputs {
            ValueOrMappingOrBytes::BytesMapping(bytes_mapping) => bytes_mapping
                .into_iter()
                .map(|(k, v)| Ok::<_, miette::Report>((k, v.extract().into_diagnostic()?)))
                .collect::<Result<HashMap<String, Vec<u8>>, _>>()
                .map_err(|err| convert_err(py, &err))?,
            ValueOrMappingOrBytes::ValueOrBytes(ValueOrMapping::Value(value)) => {
                let mut inputs = HashMap::new();
                inputs.insert(
                    "value".to_string(),
                    serde_json::to_vec(&value)
                        .into_diagnostic()
                        .map_err(|err| convert_err(py, &err))?,
                );
                inputs
            }
            ValueOrMappingOrBytes::ValueOrBytes(ValueOrMapping::Mapping(inputs)) => inputs
                .into_iter()
                .map(|(k, v)| serde_json::to_vec(&v).map(|b| (k, b)))
                .collect::<Result<HashMap<_, _>, _>>()
                .into_diagnostic()
                .map_err(|err| convert_err(py, &err))?,
        };

        let outputs = runtime::run_workflow_in_memory(workflow_graph, inputs)
            .map_err(|err| convert_err(py, &err))?;

        let mut outputs: HashMap<String, Option<Value>> = outputs
            .into_iter()
            .map(|(k, v)| Ok((k.clone(), serde_json::from_slice(&v).into_diagnostic()?)))
            .collect::<miette::Result<_>>()
            .map_err(|err| convert_err(py, &err))?;

        info!("done with outputs: {outputs:?}");
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
