/*!
This module defines the Workflow graph representation.
*/

use std::collections::HashMap;

use bitvec::vec::BitVec;
use miette::{Context, IntoDiagnostic, miette};
use portgraph::{
    Direction, LinkMut, LinkView, NodeIndex, PortGraph, PortIndex, PortMut, PortView,
    algorithms::{TopoSort, toposort_filtered},
};
use serde::{Deserialize, Serialize};

/// Possible definitions for Nodes in the [`WorkflowGraph`].
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum NodeDefinition {
    /// A node that takes input values to the Graph.
    Input {
        /// The name of the input value to the Graph.
        name: String,
    },
    /// A node that returns output values from the Graph.
    Output {},
    /// A node that loads a constant value into the Graph.
    Const {
        /// The constant value to load.
        value: serde_json::Value,
    },
    /// A node that defines a Task to be performed by a Worker in the Graph.
    Task {
        /// The name of the Worker to invoke.
        worker_name: String,
        /// The name of the Task to perform.
        task_name: String,
    },
    /// A node that defines that a Subgraph needs to be evaluated.
    Eval {},
}

/// The [`WorkflowGraph`] defines a Workflow that can be evaluated
/// by the Tierkreis runtime.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WorkflowGraph {
    graph: PortGraph,
    node_definitions: HashMap<NodeIndex, NodeDefinition>,
    port_indices: HashMap<NodeIndex, HashMap<String, PortIndex>>,
    port_names: HashMap<PortIndex, String>,
    output_node: NodeIndex,
}

impl WorkflowGraph {
    /// Create a new [`WorkflowGraph`] with an iterator of output names.
    ///
    /// The [`WorkflowGraph`] will be created with a single Output node ports
    /// of the provided names.
    pub fn new(graph_outputs: impl IntoIterator<Item = String>) -> Self {
        let graph_outputs: Vec<String> = graph_outputs.into_iter().collect();
        let mut graph = PortGraph::new();
        let idx = graph.add_node(graph_outputs.len(), 0);

        let mut port_names = HashMap::new();
        let mut output_port_indices = HashMap::new();
        let mut port_indices = HashMap::new();

        graph
            .inputs(idx)
            .zip(graph_outputs)
            .for_each(|(p, node_input)| {
                output_port_indices.insert(node_input.clone(), p);
                port_names.insert(p, node_input);
            });

        port_indices.insert(idx, output_port_indices);

        let mut node_definitions = HashMap::new();
        node_definitions.insert(idx, NodeDefinition::Output {});

        Self {
            graph,
            node_definitions,
            port_indices,
            port_names,
            output_node: idx,
        }
    }

    /// Retrieve the [`NodeIndex`] of the output node.
    #[must_use]
    pub fn output_idx(&self) -> NodeIndex {
        self.output_node
    }

    /// Retrieve the [`NodeDefinition`] for a node if it exists.
    #[must_use]
    pub fn node_definition(&self, node: NodeIndex) -> Option<&NodeDefinition> {
        self.node_definitions.get(&node)
    }

    /// Iterate over the indices of all the nodes in the grraph.
    pub fn node_ids(&self) -> impl Iterator<Item = NodeIndex> {
        self.graph.nodes_iter()
    }

    /// Add a node to the graph using a definition and the names of the input and output ports.
    pub fn add_node(
        &mut self,
        node_definition: NodeDefinition,
        inputs: impl IntoIterator<Item = String>,
        outputs: impl IntoIterator<Item = String>,
    ) -> NodeIndex {
        let inputs: Vec<String> = inputs.into_iter().collect();
        let outputs: Vec<String> = outputs.into_iter().collect();
        let idx = self.graph.add_node(inputs.len(), outputs.len());
        let mut node_port_indices = HashMap::new();
        self.graph.inputs(idx).zip(inputs).for_each(|(p, input)| {
            node_port_indices.insert(input.clone(), p);
            self.port_names.insert(p, input);
        });
        self.graph
            .outputs(idx)
            .zip(outputs)
            .for_each(|(p, output)| {
                node_port_indices.insert(output.clone(), p);
                self.port_names.insert(p, output);
            });
        self.port_indices.insert(idx, node_port_indices);
        self.node_definitions.insert(idx, node_definition);
        idx
    }

    /// Link two nodes together using their port names.
    ///
    /// # Errors
    ///
    /// Will return Err if the provided `NodeIndex` or Port names are invalid or if the internal
    /// state of the Workflow graph is invalid or corrupted.
    pub fn link_nodes_by_port_name(
        &mut self,
        from: &NodeIndex,
        from_output: &str,
        to: &NodeIndex,
        to_input: &str,
    ) -> miette::Result<()> {
        let res: miette::Result<()> = {
            let from_port = self
                .port_indices
                .get(from)
                .ok_or_else(|| miette!("Node `from` not found in port indices mapping: {from:?}"))?
                .get(from_output)
                .ok_or_else(|| {
                    miette!("Port `from_output` not found in port indices mapping: {from:?}")
                })?;
            let to_port = self
                .port_indices
                .get(to)
                .ok_or_else(|| miette!("Node `to` not found in port indices mapping: {from:?}"))?
                .get(to_input)
                .ok_or_else(|| {
                    miette!("Port `to_input` not found in port indices mapping: {from:?}")
                })?;
            self.graph
                .link_ports(*from_port, *to_port)
                .into_diagnostic()?;
            Ok(())
        };
        res.wrap_err(miette!(
            "Failed to link ports {from:?}:{from_output} -> {to:?}:{to_input}"
        ))
    }

    /// Returns an iterator of input links from a provided `NodeIndex`.
    ///
    /// The links contain the input port on the provided node and the
    /// output port of the connected node.
    pub fn input_links(
        &self,
        node: NodeIndex,
    ) -> impl Iterator<Item = (PortIndex, PortIndex)> + Clone {
        self.graph.input_links(node)
    }

    /// Returns the name corresponding to a provided `PortIndex`.
    ///
    /// # Errors
    ///
    /// Will return Err if the provided `PortIndex` is not found in the Graph.
    pub fn get_port_name(&self, port: &PortIndex) -> miette::Result<&String> {
        self.port_names
            .get(port)
            .ok_or_else(|| miette!("Could not find port name for port id: {port:?}"))
    }

    /// Returns the `NodeIndex` for a specified `PortIndex`.
    ///
    /// # Errors
    ///
    /// Will return Err if the provided `PortIndex` is not found in the Graph.
    pub fn port_node(&self, port: PortIndex) -> miette::Result<NodeIndex> {
        self.graph
            .port_node(port)
            .ok_or_else(|| miette!("Could not find node for port: {port:?}"))
    }

    /// Apply a function that returns a bool to all the input neighbours of a Node and return true
    /// if all the returned values are true and false otherwise.
    pub fn all_inputs(&self, node: NodeIndex, f: impl Fn(NodeIndex) -> bool) -> bool {
        self.graph.input_neighbours(node).all(f)
    }

    /// Return an iterator that returns the `NodeIndex` of Nodes where
    /// the supplied `filter` function returns `true`, starting at the
    /// `output_node` of the graph and then in topologically sorted order.
    pub fn toposort_filtered_from_output_node<'a>(
        &'a self,
        filter: impl Fn(NodeIndex) -> bool + 'a,
    ) -> TopoSort<'a, &'a PortGraph> {
        toposort_filtered::<_, BitVec>(
            &self.graph,
            [self.output_node],
            Direction::Incoming,
            filter,
            |_, _| true,
        )
    }
}
