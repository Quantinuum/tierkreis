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
    use std::collections::HashMap;

    use miette::{Context, Diagnostic, IntoDiagnostic};
    use num_complex::Complex64;
    use pyo3::{FromPyObject, PyErr, Python, exceptions::PyValueError, prelude::*, types::PyBytes};
    use serde::{Deserialize, Serialize};
    use tracing::info;

    use crate::{graph::LegacyWorkflowGraph, runtime};

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
            // Python already indents exception notes when rendering a traceback.
            // Keeping the note itself unindented avoids ragged output in notebooks
            // and terminals.
            add_exception_note(py, &py_err, &format!("help: {help}"));
        }
        if let Some(related) = err.related() {
            for related in related {
                add_exception_note(py, &py_err, &format!("related: {related}"));
            }
        }
        py_err
    }

    fn add_exception_note(py: Python<'_>, py_err: &PyErr, note: &str) {
        if py_err.add_note(py, note).is_err() {
            // Exception notes were added in Python 3.11. Keep diagnostics useful
            // when the abi3 extension is loaded by an older supported Python by
            // appending the note to the exception's normal message.
            let message = format!("{}\n\n{note}", py_err.value(py));
            let _ = py_err.value(py).setattr("args", (message,));
        }
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
            .wrap_err_with(|| format!("Failed to serialize workflow `{name}` to JSON"))
            .map_err(|err| convert_err(py, err))?
            .to_string();

        let legacy_workflow: LegacyWorkflowGraph = serde_json::from_str(&workflow_dump)
            .into_diagnostic()
            .wrap_err_with(|| format!("Failed to parse the serialized graph for workflow `{name}`"))
            .map_err(|err| convert_err(py, err))?;
        let workflow_graph = legacy_workflow
            .to_workflow_graph()
            .wrap_err_with(|| format!("Failed to build the runtime graph for workflow `{name}`"))
            .map_err(|err| convert_err(py, err))?;

        let inputs = match inputs {
            ValueOrMappingOrBytes::BytesMapping(bytes_mapping) => bytes_mapping
                .into_iter()
                .map(|(name, value)| {
                    let bytes = value
                        .extract()
                        .into_diagnostic()
                        .wrap_err_with(|| format!("Failed to read bytes for input `{name}`"))?;
                    Ok::<_, miette::Report>((name, bytes))
                })
                .collect::<Result<HashMap<String, Vec<u8>>, _>>()
                .map_err(|err| convert_err(py, err))?,
            ValueOrMappingOrBytes::ValueOrBytes(ValueOrMapping::Value(value)) => {
                let mut inputs = HashMap::new();
                inputs.insert(
                    "value".to_string(),
                    serde_json::to_vec(&value)
                        .into_diagnostic()
                        .wrap_err("Failed to encode workflow input `value` as JSON")
                        .map_err(|err| convert_err(py, err))?,
                );
                inputs
            }
            ValueOrMappingOrBytes::ValueOrBytes(ValueOrMapping::Mapping(inputs)) => inputs
                .into_iter()
                .map(|(name, value)| {
                    serde_json::to_vec(&value)
                        .into_diagnostic()
                        .wrap_err_with(|| {
                            format!("Failed to encode workflow input `{name}` as JSON")
                        })
                        .map(|bytes| (name, bytes))
                })
                .collect::<miette::Result<HashMap<_, _>>>()
                .map_err(|err| convert_err(py, err))?,
        };

        let outputs = runtime::run_workflow_in_memory(workflow_graph, inputs)
            .wrap_err_with(|| format!("Workflow `{name}` failed"))
            .map_err(|err| convert_err(py, err))?;

        let mut outputs: HashMap<String, Option<Value>> = outputs
            .into_iter()
            .map(|(name, value)| {
                let value = serde_json::from_slice(&value)
                    .into_diagnostic()
                    .wrap_err_with(|| {
                        format!("Failed to decode workflow output `{name}` as JSON")
                    })?;
                Ok((name, value))
            })
            .collect::<miette::Result<_>>()
            .map_err(|err| convert_err(py, err))?;

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

    #[cfg(test)]
    mod tests {
        use miette::miette;

        use super::*;

        #[test]
        fn python_error_preserves_context_chain_and_formats_help_note() {
            Python::initialize();
            Python::attach(|py| {
                let report = miette!(help = "Check the workflow input.", "invalid JSON")
                    .wrap_err("Failed to decode workflow input `value`");
                let py_err = convert_err(py, report);

                let cause = py_err.cause(py).expect("missing Python exception cause");
                assert_eq!(cause.value(py).to_string(), "invalid JSON");

                if let Ok(notes) = py_err.value(py).getattr("__notes__") {
                    let notes: Vec<String> = notes.extract().expect("invalid exception notes");
                    assert_eq!(
                        py_err.value(py).to_string(),
                        "Failed to decode workflow input `value`"
                    );
                    assert_eq!(notes, ["help: Check the workflow input."]);
                } else {
                    assert_eq!(
                        py_err.value(py).to_string(),
                        "Failed to decode workflow input `value`\n\nhelp: Check the workflow input."
                    );
                }
            });
        }
    }
}
