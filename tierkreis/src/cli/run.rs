use std::{collections::HashMap, fs, path::Path, process::Command};

use miette::{Context, IntoDiagnostic, bail, miette};

use crate::{
    graph::{LegacyWorkflowGraph, WorkflowGraph},
    runtime::{RuntimeConfig, run_workflow_once},
};

pub(super) struct Options<'a> {
    pub workflow: Option<&'a Path>,
    pub python: Option<&'a str>,
    pub inputs: &'a Path,
    pub name: Option<String>,
    pub print_output: bool,
    pub config: Option<&'a Path>,
}

fn load_serialized_graph(bytes: &[u8]) -> miette::Result<WorkflowGraph> {
    if let Ok(graph) = serde_json::from_slice::<WorkflowGraph>(bytes) {
        return Ok(graph);
    }
    serde_json::from_slice::<LegacyWorkflowGraph>(bytes)
        .into_diagnostic()
        .wrap_err("workflow is neither a current nor legacy Tierkreis graph")?
        .to_workflow_graph()
}

fn python_graph(specifier: &str) -> miette::Result<Vec<u8>> {
    let mut command = if let Ok(uv) = which::which("uv") {
        let mut command = Command::new(uv);
        command.args(["run", "--active", "--no-project", "python"]);
        command
    } else {
        let python = which::which("python3")
            .or_else(|_| which::which("python"))
            .into_diagnostic()
            .wrap_err("Python graph loading requires `uv`, `python3`, or `python`")?;
        Command::new(python)
    };
    let output = command
        .args(["-m", "tierkreis.cli.bridge", "export-graph", specifier])
        .output()
        .into_diagnostic()
        .wrap_err("failed to start the Python graph adapter")?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!(
            "Python graph adapter failed with {}: {stderr}",
            output.status
        );
    }
    Ok(output.stdout)
}

fn load_inputs(path: &Path) -> miette::Result<HashMap<String, Vec<u8>>> {
    let bytes = fs::read(path)
        .into_diagnostic()
        .wrap_err_with(|| format!("failed to read inputs from {}", path.display()))?;
    let values: HashMap<String, serde_json::Value> = serde_json::from_slice(&bytes)
        .into_diagnostic()
        .wrap_err("workflow inputs must be a JSON object")?;
    values
        .into_iter()
        .map(|(name, value)| {
            serde_json::to_vec(&value)
                .map(|bytes| (name, bytes))
                .into_diagnostic()
        })
        .collect()
}

pub(super) fn run(options: Options<'_>) -> miette::Result<()> {
    let graph_bytes = match (options.workflow, options.python) {
        (Some(path), None) => fs::read(path)
            .into_diagnostic()
            .wrap_err_with(|| format!("failed to read workflow from {}", path.display()))?,
        (None, Some(specifier)) => python_graph(specifier)?,
        (Some(_), Some(_)) => bail!("pass either WORKFLOW or --python, not both"),
        (None, None) => bail!("a serialized WORKFLOW or --python graph is required"),
    };
    let graph = load_serialized_graph(&graph_bytes)?;
    let inputs = load_inputs(options.inputs)?;
    let config = match options.config {
        Some(path) => RuntimeConfig::from_file(path)?,
        None => RuntimeConfig::load()?,
    };
    let result = run_workflow_once(&config, options.name, graph, inputs)?;
    println!("Workflow ID: {}", result.workflow_id);
    println!("Run ID: {}", result.run_id);
    if options.print_output {
        let outputs = result
            .outputs
            .into_iter()
            .map(|(name, bytes)| {
                let value = serde_json::from_slice::<serde_json::Value>(&bytes)
                    .map_err(|error| miette!("output `{name}` is not JSON: {error}"))?;
                Ok((name, value))
            })
            .collect::<miette::Result<serde_json::Map<_, _>>>()?;
        println!(
            "{}",
            serde_json::to_string_pretty(&outputs).into_diagnostic()?
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inputs_are_encoded_individually() {
        let file = tempfile::NamedTempFile::new().unwrap();
        fs::write(file.path(), r#"{"a": 1, "b": "value"}"#).unwrap();
        let inputs = load_inputs(file.path()).unwrap();
        assert_eq!(inputs["a"], b"1");
        assert_eq!(inputs["b"], br#""value""#);
    }

    #[test]
    fn inputs_must_be_an_object() {
        let file = tempfile::NamedTempFile::new().unwrap();
        fs::write(file.path(), "[1, 2]").unwrap();
        assert!(load_inputs(file.path()).is_err());
    }
}
