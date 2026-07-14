/*! This is a graph builder utility module that is currently just used for
building test workflow graphs internally.
*/
#![allow(missing_docs)]
use miette::IntoDiagnostic;
use portgraph::NodeIndex;
use serde::Serialize;

use crate::graph::{NodeDefinition, WorkflowGraph};

pub fn workflow<'a>(outputs: impl IntoIterator<Item = &'a str>) -> WorkflowGraph {
    WorkflowGraph::new(outputs.into_iter().map(ToString::to_string))
}

pub fn input<'a>(graph: &mut WorkflowGraph, name: &'a str) -> (NodeIndex, &'a str) {
    let node = graph.add_node(
        NodeDefinition::Input {
            name: name.to_string(),
        },
        [],
        [name.to_string()],
    );
    (node, name)
}

#[must_use] 
pub fn output<'a>(graph: &WorkflowGraph, name: &'a str) -> (NodeIndex, &'a str) {
    (graph.output_idx(), name)
}

pub fn constant(
    graph: &mut WorkflowGraph,
    value: impl Serialize,
) -> miette::Result<(NodeIndex, &'static str)> {
    let node = graph.add_node(
        NodeDefinition::Const {
            value: serde_json::to_value(value).into_diagnostic()?,
        },
        [],
        ["value".to_string()],
    );
    Ok((node, "value"))
}

pub fn task<'a>(
    graph: &mut WorkflowGraph,
    worker: &str,
    task: &str,
    inputs: impl IntoIterator<Item = &'a str>,
    outputs: impl IntoIterator<Item = &'a str>,
) -> NodeIndex {
    graph.add_node(
        NodeDefinition::Task {
            worker_name: worker.to_string(),
            task_name: task.to_string(),
        },
        inputs.into_iter().map(ToString::to_string),
        outputs.into_iter().map(ToString::to_string),
    )
}

pub fn if_else(graph: &mut WorkflowGraph) -> NodeIndex {
    graph.add_node(
        NodeDefinition::IfElse {},
        [
            "pred".to_string(),
            "if_true".to_string(),
            "if_false".to_string(),
        ],
        ["value".to_string()],
    )
}

pub fn eager_if_else(graph: &mut WorkflowGraph) -> NodeIndex {
    graph.add_node(
        NodeDefinition::IfElse {},
        [
            "pred".to_string(),
            "if_true".to_string(),
            "if_false".to_string(),
        ],
        ["value".to_string()],
    )
}

pub fn eval<'a>(
    graph: &mut WorkflowGraph,
    inputs: impl IntoIterator<Item = &'a str>,
    outputs: impl IntoIterator<Item = &'a str>,
) -> NodeIndex {
    graph.add_node(
        NodeDefinition::Eval {},
        ["graph".to_string()]
            .into_iter()
            .chain(inputs.into_iter().map(ToString::to_string)),
        outputs.into_iter().map(ToString::to_string),
    )
}

pub fn loop_node<'a>(
    graph: &mut WorkflowGraph,
    inputs: impl IntoIterator<Item = &'a str>,
    outputs: impl IntoIterator<Item = &'a str>,
) -> NodeIndex {
    graph.add_node(
        NodeDefinition::Loop {},
        ["graph".to_string()]
            .into_iter()
            .chain(inputs.into_iter().map(ToString::to_string)),
        outputs.into_iter().map(ToString::to_string),
    )
}

pub fn map_node<'a>(
    graph: &mut WorkflowGraph,
    mapped_inputs: impl IntoIterator<Item = &'a str>,
    other_inputs: impl IntoIterator<Item = &'a str>,
    outputs: impl IntoIterator<Item = &'a str>,
) -> NodeIndex {
    let mapped: Vec<&str> = mapped_inputs.into_iter().collect();
    graph.add_node(
        NodeDefinition::Map {
            mapped_ports: mapped.iter().map(ToString::to_string).collect(),
        },
        ["graph".to_string()]
            .into_iter()
            .chain(mapped.into_iter().map(ToString::to_string))
            .chain(other_inputs.into_iter().map(ToString::to_string)),
        outputs.into_iter().map(ToString::to_string),
    )
}

pub fn link<U, V>(
    graph: &mut WorkflowGraph,
    from: (NodeIndex, U),
    to: (NodeIndex, V),
) -> miette::Result<()>
where
    U: AsRef<str>,
    V: AsRef<str>,
{
    let (from, from_output) = from;
    let (to, to_input) = to;
    graph.link_nodes_by_port_name(from, from_output.as_ref(), to, to_input.as_ref())
}
