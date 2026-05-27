/*! This is the library module for the rust components of the Tierkreis
Workflow Management system.
*/
#![warn(missing_docs)]
#![warn(clippy::all, clippy::pedantic)]

pub mod asset_storage;
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
    use pyo3::{PyErr, exceptions::PyValueError, prelude::*};
    use serde::{Deserialize, Serialize};

    use crate::{graph::LegacyWorkflowGraph, runtime};

    /// Wrapper type for bridging miette and pyo3 errors.
    struct MietteReport(miette::Report);

    impl From<MietteReport> for PyErr {
        fn from(err: MietteReport) -> Self {
            PyValueError::new_err(err.0.to_string())
        }
    }

    impl From<miette::Report> for MietteReport {
        fn from(report: miette::Report) -> Self {
            Self(report)
        }
    }

    #[derive(FromPyObject, Serialize, Deserialize)]
    #[serde(untagged)]
    enum Inputs {
        Mapping(HashMap<String, Value>),
        Value(Value),
    }

    #[derive(FromPyObject, Serialize, Deserialize)]
    #[serde(untagged)]
    enum Value {
        Int(i64),
        Float(f64),
        String(String),
        List(Vec<Value>),
        Dict(HashMap<String, Value>),
        Bytes(Vec<u8>),
    }

    #[pyfunction]
    fn run_workflow(workflow: &Bound<'_, PyAny>, inputs: Inputs) -> Result<(), MietteReport> {
        let workflow_dump: String = workflow
            .call_method0("model_dump_json")
            .into_diagnostic()?
            .to_string();

        let legacy_workflow: LegacyWorkflowGraph =
            serde_json::from_str(&workflow_dump).into_diagnostic()?;
        let workflow_graph = legacy_workflow.to_workflow_graph()?;

        let inputs = match inputs {
            Inputs::Value(value) => {
                let mut inputs = HashMap::new();
                inputs.insert(
                    "value".to_string(),
                    serde_json::to_vec(&value).into_diagnostic()?,
                );
                inputs
            }
            Inputs::Mapping(inputs) => inputs
                .into_iter()
                .map(|(k, v)| serde_json::to_vec(&v).map(|b| (k, b)))
                .collect::<Result<HashMap<_, _>, _>>()
                .into_diagnostic()?,
        };

        let outputs = runtime::run_workflow_in_memory(&workflow_graph, inputs)?;

        let outputs: HashMap<String, serde_json::Value> = outputs
            .into_iter()
            .map(|(k, v)| Ok((k.clone(), serde_json::from_slice(&v).into_diagnostic()?)))
            .collect::<Result<_, MietteReport>>()?;

        println!("{outputs:?}");

        Ok(())
    }
}
