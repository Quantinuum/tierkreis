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
    use std::{collections::HashMap, sync::Arc};

    use miette::IntoDiagnostic;
    use num_complex::Complex64;
    use pyo3::{FromPyObject, PyErr, Python, exceptions::PyValueError, prelude::*};
    use serde::{Deserialize, Serialize};
    use tracing::info;

    use crate::{graph::LegacyWorkflowGraph, runtime};

    #[pymodule_init]
    fn init(_m: &Bound<'_, PyModule>) -> PyResult<()> {
        // console_subscriber::init();
        tracing_subscriber::fmt().compact().init();
        Ok(())
    }

    fn convert_err(py: Python<'_>, err: miette::Report) -> PyErr {
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

    #[derive(Debug, FromPyObject, IntoPyObject, Serialize, Deserialize)]
    #[serde(untagged)]
    enum ValueOrMapping {
        Mapping(HashMap<String, Value>),
        Value(Value),
    }

    #[derive(Debug, FromPyObject, IntoPyObject, Serialize, Deserialize)]
    #[serde(untagged)]
    enum Value {
        Bool(bool),
        Int(i64),
        Float(f64),
        String(String),
        Complex(Complex64),
        List(Vec<Value>),
        // TODO: Bytes pass-through is a bit unpredictable.
        Bytes(Vec<u8>),
        Dict(HashMap<String, Value>),
    }

    // If we have something that looks like bytes, we should "just" pass
    // it transparently.
    fn serialize_value(value: &Value) -> Result<Vec<u8>, serde_json::Error> {
        match value {
            // Value::Bytes(bytes) => Ok(bytes.clone()),
            _ => serde_json::to_vec(&value),
        }
    }

    #[pyfunction]
    fn run_workflow(
        py: Python<'_>,
        name: &str,
        workflow: &Bound<'_, PyAny>,
        inputs: ValueOrMapping,
    ) -> PyResult<ValueOrMapping> {
        info!("starting workflow: '{name}'");
        let workflow_dump: String = workflow
            .call_method0("model_dump_json")
            .into_diagnostic()
            .map_err(|err| convert_err(py, err))?
            .to_string();

        let legacy_workflow: LegacyWorkflowGraph = serde_json::from_str(&workflow_dump)
            .into_diagnostic()
            .map_err(|err| convert_err(py, err))?;
        let workflow_graph = Arc::new(
            legacy_workflow
                .to_workflow_graph()
                .map_err(|err| convert_err(py, err))?,
        );
        println!("{}", serde_json::to_string_pretty(&workflow_graph).unwrap());

        let inputs = match inputs {
            ValueOrMapping::Value(value) => {
                let mut inputs = HashMap::new();
                inputs.insert(
                    "value".to_string(),
                    serialize_value(&value)
                        .into_diagnostic()
                        .map_err(|err| convert_err(py, err))?,
                );
                inputs
            }
            ValueOrMapping::Mapping(inputs) => inputs
                .into_iter()
                .map(|(k, v)| serialize_value(&v).map(|b| (k, b)))
                .collect::<Result<HashMap<_, _>, _>>()
                .into_diagnostic()
                .map_err(|err| convert_err(py, err))?,
        };

        let outputs = runtime::run_workflow_in_memory(workflow_graph, inputs)
            .map_err(|err| convert_err(py, err))?;

        let mut outputs: HashMap<String, Value> = outputs
            .into_iter()
            .map(|(k, v)| Ok((k.clone(), serde_json::from_slice(&v).into_diagnostic()?)))
            .collect::<miette::Result<_>>()
            .map_err(|err| convert_err(py, err))?;

        info!("done with outputs: {outputs:?}");
        if outputs.len() == 1 && outputs.contains_key("value") {
            Ok(ValueOrMapping::Value(
                outputs.remove("value").expect("No single output value"),
            ))
        } else {
            Ok(ValueOrMapping::Mapping(outputs))
        }
    }
}
