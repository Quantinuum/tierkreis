/*!
This module defines the Workflow graph representation.
*/

use std::collections::{HashMap, HashSet};

use bitvec::vec::BitVec;
use miette::{Context, IntoDiagnostic, miette};
use portgraph::{
    Direction, LinkMut, LinkView, MultiPortGraph, NodeIndex, PortIndex, PortMut, PortView,
    algorithms::{TopoSort, toposort_filtered},
    multiportgraph::SubportIndex,
};
use serde::{Deserialize, Serialize};

/// Possible definitions for Nodes in the [`WorkflowGraph`].
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
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
    /// A node that conditionally returns one of two values depending on a boolean value.
    ///
    /// Unlike `EagerIfElse`, only the branch for the returned value should be computed.
    IfElse {},
    /// A node that conditionally returns one of two values depending on a boolean value.
    ///
    /// Unlike `IfElse`, both branches should be computed when possible.
    EagerIfElse {},
    /// A node that defines a Task to be performed by a Worker in the Graph.
    Task {
        /// The name of the Worker to invoke.
        worker_name: String,
        /// The name of the Task to perform.
        task_name: String,
    },
    /// A node that defines that a Subgraph needs to be evaluated.
    Eval {},
    /// A node that defines that a Subgraph needs to be evaluated repeatedly until a condition is met.
    Loop {},
    /// A node that defines that a Subgraph needs to be evaluated across multiple inputs.
    Map {
        /// The input ports of the Map node that are mapped over.
        mapped_ports: HashSet<String>,
    },
}

/// The [`WorkflowGraph`] defines a Workflow that can be evaluated
/// by the Tierkreis runtime.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WorkflowGraph {
    graph: MultiPortGraph,
    node_definitions: HashMap<NodeIndex, NodeDefinition>,
    input_port_indices: HashMap<NodeIndex, HashMap<String, PortIndex>>,
    output_port_indices: HashMap<NodeIndex, HashMap<String, PortIndex>>,
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
        let mut graph = MultiPortGraph::new();
        let idx = graph.add_node(graph_outputs.len(), 0);

        let mut port_names = HashMap::new();
        let mut output_node_input_port_indices = HashMap::new();
        let mut input_port_indices = HashMap::new();
        let output_port_indices = HashMap::new();

        graph
            .inputs(idx)
            .zip(graph_outputs)
            .for_each(|(port, node_input)| {
                output_node_input_port_indices.insert(node_input.clone(), port);
                port_names.insert(port, node_input);
            });

        input_port_indices.insert(idx, output_node_input_port_indices);

        let mut node_definitions = HashMap::new();
        node_definitions.insert(idx, NodeDefinition::Output {});

        Self {
            graph,
            node_definitions,
            input_port_indices,
            output_port_indices,
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

    /// Iterate over the indices of all the nodes in the graph.
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

        let mut node_input_port_indices = HashMap::new();
        let mut node_output_port_indices = HashMap::new();

        self.graph
            .inputs(idx)
            .zip(inputs)
            .for_each(|(port, input)| {
                node_input_port_indices.insert(input.clone(), port);
                self.port_names.insert(port, input);
            });
        self.input_port_indices.insert(idx, node_input_port_indices);

        self.graph
            .outputs(idx)
            .zip(outputs)
            .for_each(|(port, output)| {
                node_output_port_indices.insert(output.clone(), port);
                self.port_names.insert(port, output);
            });
        self.output_port_indices
            .insert(idx, node_output_port_indices);

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
                .output_port_indices
                .get(from)
                .ok_or_else(|| miette!("Node `from` not found in port indices mapping: {from:?}"))?
                .get(from_output)
                .ok_or_else(|| {
                    miette!("Port `from_output` not found in port indices mapping: {from:?}")
                })?;
            let to_port = self
                .input_port_indices
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
        res.wrap_err_with(|| {
            miette!("Failed to link ports {from:?}:{from_output} -> {to:?}:{to_input}")
        })
    }

    /// Returns an iterator of input links from a provided `NodeIndex`.
    ///
    /// The links contain the input port on the provided node and the
    /// output port of the connected node.
    pub fn input_links(
        &self,
        node: NodeIndex,
    ) -> impl Iterator<Item = (SubportIndex, SubportIndex)> + Clone {
        self.graph.input_links(node)
    }

    /// Returns the `NodeIndex` and `PortIndex` of the port linked to the provided
    /// input port name on a particular node.
    ///
    /// # Errors
    ///
    /// Will return Err if the provided `PortIndex` is not found in the Graph or
    /// if the port is not linked to a port on another node.
    pub fn connected_input_by_port_name(
        &self,
        node: &NodeIndex,
        port_name: &str,
    ) -> miette::Result<(NodeIndex, PortIndex)> {
        let port = self.get_input_port_index(node, port_name)?;
        self.connected_input(port)
    }

    /// Returns a the `NodeIndex` and `PortIndex` of the port linked to the provided
    /// input port.
    ///
    /// # Errors
    ///
    /// Will return Err if the provided `PortIndex` is not found in the Graph or
    /// if the port is not linked to a port on another node.
    pub fn connected_input(&self, port: &PortIndex) -> miette::Result<(NodeIndex, PortIndex)> {
        let connected_port = self
            .graph
            .port_link(*port)
            .ok_or_else(|| miette!("Port link not found"))?
            .port();
        let connected_node = self
            .graph
            .port_node(connected_port)
            .ok_or_else(|| miette!("Could not find node for port: {connected_port:?}"))?;
        Ok((connected_node, connected_port))
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

    /// Returns the `PortIndex` corresponding to a provided name.
    ///
    /// # Errors
    ///
    /// Will return Err if the provided node index is not found or if the port name is not found.
    pub fn get_input_port_index(
        &self,
        node: &NodeIndex,
        port_name: &str,
    ) -> miette::Result<&PortIndex> {
        self.input_port_indices
            .get(node)
            .ok_or_else(|| miette!("Could not find node with index: {node:?}"))?
            .get(port_name)
            .ok_or_else(|| miette!("Could not find port id for port name: {port_name}"))
    }

    /// Returns the `NodeIndex` for a specified `PortIndex`.
    ///
    /// # Errors
    ///
    /// Will return Err if the provided `PortIndex` is not found in the Graph.
    pub fn port_node(&self, port: SubportIndex) -> miette::Result<NodeIndex> {
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
        node_filter: impl Fn(NodeIndex) -> bool + 'a,
        port_filter: impl Fn(NodeIndex, PortIndex) -> bool + 'a,
    ) -> TopoSort<'a, &'a MultiPortGraph> {
        toposort_filtered::<_, BitVec>(
            &self.graph,
            [self.output_node],
            Direction::Incoming,
            node_filter,
            port_filter,
        )
    }
}

type ValueRef = (i32, String);

// Note that many fields are ignored as they are not necessary for rebuilding
// the graph as in most cases the ports are standardized by the node type.
#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
enum LegacyNodeDef {
    #[serde(rename = "function")]
    Func {
        function_name: String,
        inputs: HashMap<String, ValueRef>,
        outputs: HashMap<String, Vec<u32>>,
    },
    #[serde(rename = "eval")]
    Eval {
        graph: ValueRef,
        inputs: HashMap<String, ValueRef>,
        outputs: HashMap<String, Vec<u32>>,
    },
    #[serde(rename = "loop")]
    Loop {
        body: ValueRef,
        inputs: HashMap<String, ValueRef>,
        continue_port: String,
        outputs: HashMap<String, Vec<u32>>,
    },
    #[serde(rename = "map")]
    Map {
        body: ValueRef,
        inputs: HashMap<String, ValueRef>,
        outputs: HashMap<String, Vec<u32>>,
    },
    #[serde(rename = "const")]
    Const { value: serde_json::Value },
    #[serde(rename = "ifelse")]
    IfElse {
        pred: ValueRef,
        if_true: ValueRef,
        if_false: ValueRef,
    },
    #[serde(rename = "eifelse")]
    EagerIfElse {
        pred: ValueRef,
        if_true: ValueRef,
        if_false: ValueRef,
    },
    #[serde(rename = "input")]
    Input { name: String },
    #[serde(rename = "output")]
    Output { inputs: HashMap<String, ValueRef> },
}

/// A serializable structure for the previous workflow graph format.
#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct LegacyWorkflowGraph {
    nodes: Vec<LegacyNodeDef>,
    graph_inputs: Vec<String>,
    graph_output_idx: u32,
}

impl LegacyWorkflowGraph {
    /// Convert the `LegacyWorkflowGraph` into a `WorkflowGraph` that can be run by the Orchestrator.
    ///
    /// # Errors
    ///
    /// Will return `Err` if the graph is malformed and the conversion fails.
    pub fn to_workflow_graph(self) -> miette::Result<WorkflowGraph> {
        let mut state = ConversionState::new(self.nodes.len());

        for node in self.nodes {
            state.convert_node(node)?;
        }

        state.link_ports()?;
        state.remove_fold_and_unfold()?;
        state.migrate_wildcard_ports();

        Ok(WorkflowGraph {
            graph: state.graph,
            node_definitions: state.node_definitions,
            input_port_indices: state.input_port_indices,
            output_port_indices: state.output_port_indices,
            port_names: state.port_names,
            output_node: state
                .output_node
                .ok_or_else(|| miette!("Output node not found"))?,
        })
    }
}

struct ConversionState {
    pub graph: MultiPortGraph,
    pub node_definitions: HashMap<NodeIndex, NodeDefinition>,
    pub input_port_indices: HashMap<NodeIndex, HashMap<String, PortIndex>>,
    pub output_port_indices: HashMap<NodeIndex, HashMap<String, PortIndex>>,
    pub port_names: HashMap<PortIndex, String>,
    pub to_link: HashMap<PortIndex, ValueRef>,
    pub output_node: Option<NodeIndex>,
}

impl ConversionState {
    fn new(total_nodes: usize) -> Self {
        // Rought heuristic that we will likely have around 2 ports per node.
        let graph = MultiPortGraph::with_capacity(total_nodes, total_nodes * 2);

        let node_definitions = HashMap::with_capacity(total_nodes);
        let input_port_indices = HashMap::with_capacity(total_nodes);
        let output_port_indices = HashMap::with_capacity(total_nodes);
        let port_names = HashMap::new();
        let to_link = HashMap::new();
        let output_node = None;

        Self {
            graph,
            node_definitions,
            input_port_indices,
            output_port_indices,
            port_names,
            to_link,
            output_node,
        }
    }

    fn remove_fold_and_unfold(&mut self) -> Result<(), miette::Error> {
        let mut remove_nodes: Vec<_> = self
            .node_definitions
            .iter()
            .filter(|(_idx, node)| {
                matches!(
                    node,
                    &NodeDefinition::Task {
                        worker_name,
                        task_name,
                    } if worker_name == "builtins" && task_name == "unfold_values"
                )
            })
            .map(|(idx, _node)| {
                ConversionState::relink_node(
                    *idx,
                    &mut self.graph,
                    &mut self.input_port_indices,
                    &mut self.output_port_indices,
                    &mut self.port_names,
                )
            })
            .collect::<miette::Result<_>>()?;

        remove_nodes.extend(
            self.node_definitions
                .iter()
                .filter(|(_idx, node)| {
                    matches!(
                        node,
                        &NodeDefinition::Task {
                            worker_name,
                            task_name,
                        } if worker_name == "builtins" && task_name == "fold_values"
                    )
                })
                .map(|(idx, _node)| {
                    ConversionState::relink_node(
                        *idx,
                        &mut self.graph,
                        &mut self.input_port_indices,
                        &mut self.output_port_indices,
                        &mut self.port_names,
                    )
                })
                .collect::<miette::Result<Vec<_>>>()?,
        );

        for node in remove_nodes {
            self.node_definitions.remove(&node);
        }

        Ok(())
    }

    fn relink_node(
        idx: NodeIndex,
        graph: &mut MultiPortGraph,
        input_port_indices: &mut HashMap<NodeIndex, HashMap<String, PortIndex>>,
        output_port_indices: &mut HashMap<NodeIndex, HashMap<String, PortIndex>>,
        port_names: &mut HashMap<PortIndex, String>,
    ) -> Result<NodeIndex, miette::Error> {
        // Re-link the single input and output
        let input = graph
            .input(idx, 0)
            .ok_or_else(|| miette!("Missing input port or node"))?;
        let input_link = graph
            .port_link(input)
            .ok_or_else(|| miette!("Missing link!"))?;
        let output = graph
            .output(idx, 0)
            .ok_or_else(|| miette!("Missing input port or node"))?;
        let output_link = graph
            .port_link(output)
            .ok_or_else(|| miette!("Missing link!"))?;
        graph
            .link_ports(input_link.into(), output_link.into())
            .map_err(|err| miette!("Failed to link ports: {err}"))?;

        for port in graph.all_ports(idx) {
            port_names.remove(&port);
        }
        graph.remove_node(idx);
        input_port_indices.remove(&idx);
        output_port_indices.remove(&idx);

        Ok(idx)
    }

    fn migrate_wildcard_ports(&mut self) {
        self.node_definitions
            .iter_mut()
            .filter_map(|(_, node_definition)| {
                if let NodeDefinition::Map { mapped_ports } = node_definition {
                    Some(mapped_ports)
                } else {
                    None
                }
            })
            .for_each(|mapped_ports| {
                *mapped_ports = mapped_ports
                    .iter()
                    .map(|port_name| {
                        let mut new_name = port_name.clone();
                        if port_name == "*" {
                            new_name = "value".to_string();
                        } else if port_name.ends_with("-*") {
                            new_name = port_name.strip_suffix("-*").unwrap().to_string();
                        }
                        new_name
                    })
                    .collect();
            });

        self.input_port_indices.values_mut().for_each(|value| {
            *value = value
                .iter()
                .map(|(port_name, v)| {
                    let mut new_name = port_name.clone();
                    if port_name == "*" {
                        new_name = "value".to_string();
                    } else if port_name.ends_with("-*") {
                        new_name = port_name.strip_suffix("-*").unwrap().to_string();
                    }
                    (new_name, *v)
                })
                .collect();
        });

        self.output_port_indices.values_mut().for_each(|value| {
            *value = value
                .iter()
                .map(|(port_name, v)| {
                    let mut new_name = port_name.clone();
                    if port_name == "*" {
                        new_name = "value".to_string();
                    } else if port_name.ends_with("-*") {
                        new_name = port_name.strip_suffix("-*").unwrap().to_string();
                    }
                    (new_name, *v)
                })
                .collect();
        });

        self.port_names.values_mut().for_each(|port_name| {
            if port_name == "*" {
                *port_name = "value".to_string();
            } else if port_name.ends_with("-*") {
                *port_name = port_name.strip_suffix("-*").unwrap().to_string();
            }
        });
    }

    fn link_ports(&mut self) -> Result<(), miette::Error> {
        for (in_port, (from_node, from_port_name)) in &self.to_link {
            // A negative index here signifies that port is intended to
            // be a self-referential link.
            if *from_node == -1 {
                continue;
            }

            let node_index: usize = TryFrom::<i32>::try_from(*from_node).into_diagnostic()?;
            let out_port = self
                .output_port_indices
                .get(&NodeIndex::new(node_index))
                .ok_or_else(|| miette!("Could not find node at: {}", from_node))?
                .get(from_port_name)
                .ok_or_else(|| {
                    miette!(
                        "Could not find port name: {} on node: {}",
                        from_port_name,
                        from_node
                    )
                })?;

            self.graph
                .link_ports(*out_port, *in_port)
                .into_diagnostic()?;
        }

        Ok(())
    }

    fn convert_node(&mut self, node: LegacyNodeDef) -> Result<(), miette::Error> {
        match node {
            LegacyNodeDef::Func {
                function_name,
                inputs,
                outputs,
            } => {
                self.convert_func(&function_name, inputs, outputs)?;
            }
            LegacyNodeDef::Eval {
                graph: graph_source,
                inputs,
                outputs,
            } => {
                self.convert_eval(graph_source, inputs, outputs);
            }
            LegacyNodeDef::Loop {
                body,
                continue_port: _continue_port,
                inputs,
                outputs,
            } => {
                self.convert_loop(body, inputs, outputs);
            }
            LegacyNodeDef::Map {
                body,
                inputs,
                outputs,
            } => {
                self.convert_map(body, inputs, outputs);
            }
            LegacyNodeDef::Const { value } => {
                self.convert_const(value);
            }
            LegacyNodeDef::IfElse {
                pred,
                if_true,
                if_false,
            } => {
                self.convert_if_else(pred, if_true, if_false);
            }
            LegacyNodeDef::EagerIfElse {
                pred,
                if_true,
                if_false,
            } => {
                self.convert_eager_if_else(pred, if_true, if_false);
            }
            LegacyNodeDef::Input { name } => {
                self.convert_input(name);
            }
            LegacyNodeDef::Output { inputs } => {
                self.convert_output(inputs);
            }
        }

        Ok(())
    }

    fn convert_output(&mut self, inputs: HashMap<String, ValueRef>) {
        let incoming = inputs.len();
        let node_index = self.graph.add_node(incoming, 0);
        self.output_node = Some(node_index);

        self.node_definitions
            .insert(node_index, NodeDefinition::Output {});

        self.build_inputs(inputs, node_index);
    }

    fn convert_input(&mut self, name: String) {
        let node_index = self.graph.add_node(0, 1);

        self.node_definitions
            .insert(node_index, NodeDefinition::Input { name: name.clone() });

        self.build_outputs([name], node_index);
    }

    fn convert_eager_if_else(&mut self, pred: ValueRef, if_true: ValueRef, if_false: ValueRef) {
        let node_index = self.graph.add_node(3, 1);

        self.node_definitions
            .insert(node_index, NodeDefinition::EagerIfElse {});

        self.build_inputs(
            [
                ("pred".to_string(), pred),
                ("if_true".to_string(), if_true),
                ("if_false".to_string(), if_false),
            ],
            node_index,
        );
        self.build_outputs(["value".to_string()], node_index);
    }

    fn convert_if_else(&mut self, pred: ValueRef, if_true: ValueRef, if_false: ValueRef) {
        let node_index = self.graph.add_node(3, 1);

        self.node_definitions
            .insert(node_index, NodeDefinition::IfElse {});

        self.build_inputs(
            [
                ("pred".to_string(), pred),
                ("if_true".to_string(), if_true),
                ("if_false".to_string(), if_false),
            ],
            node_index,
        );
        self.build_outputs(["value".to_string()], node_index);
    }

    fn convert_const(&mut self, value: serde_json::Value) {
        let node_index = self.graph.add_node(0, 1);

        self.node_definitions
            .insert(node_index, NodeDefinition::Const { value });

        self.build_outputs(["value".to_string()], node_index);
    }

    fn convert_eval(
        &mut self,
        graph_source: ValueRef,
        inputs: HashMap<String, ValueRef>,
        outputs: HashMap<String, Vec<u32>>,
    ) {
        let incoming = inputs.len();
        let outgoing = outputs.len();
        let node_index = self.graph.add_node(incoming + 1, outgoing);

        self.node_definitions
            .insert(node_index, NodeDefinition::Eval {});

        self.build_inputs(
            [("graph".to_string(), graph_source)]
                .into_iter()
                .chain(inputs),
            node_index,
        );
        self.build_outputs(outputs.into_keys(), node_index);
    }

    fn convert_loop(
        &mut self,
        graph_source: ValueRef,
        inputs: HashMap<String, ValueRef>,
        outputs: HashMap<String, Vec<u32>>,
    ) {
        let incoming = inputs.len();
        let outgoing = outputs.len();
        let node_index = self.graph.add_node(incoming + 1, outgoing);

        self.node_definitions
            .insert(node_index, NodeDefinition::Loop {});

        self.build_inputs(
            [("graph".to_string(), graph_source)]
                .into_iter()
                .chain(inputs),
            node_index,
        );
        self.build_outputs(outputs.into_keys(), node_index);
    }

    fn convert_map(
        &mut self,
        graph_source: ValueRef,
        inputs: HashMap<String, ValueRef>,
        outputs: HashMap<String, Vec<u32>>,
    ) {
        let incoming = inputs.len();
        let outgoing = outputs.len();
        let node_index = self.graph.add_node(incoming + 1, outgoing);

        let mut mapped_ports = HashSet::new();
        for (name, (_, connected)) in &inputs {
            // Ports containing a `*` are mapped over.
            if connected == "*" || connected.ends_with("-*") {
                mapped_ports.insert(name.clone());
            }
        }

        self.node_definitions
            .insert(node_index, NodeDefinition::Map { mapped_ports });

        self.build_inputs(
            [("graph".to_string(), graph_source)]
                .into_iter()
                .chain(inputs),
            node_index,
        );
        self.build_outputs(outputs.into_keys(), node_index);
    }

    fn convert_func(
        &mut self,
        function_name: &str,
        inputs: HashMap<String, ValueRef>,
        outputs: HashMap<String, Vec<u32>>,
    ) -> Result<(), miette::Error> {
        let incoming = inputs.len();
        let outgoing = outputs.len();
        let node_index = self.graph.add_node(incoming, outgoing);
        let mut parts = function_name.split('.');
        let worker_name = parts.next().ok_or_else(|| {
            miette!(
                "Could not find worker name in function name: {}",
                function_name
            )
        })?;
        let task_name = parts.next().ok_or_else(|| {
            miette!(
                "Could not find task name in function name: {}",
                function_name
            )
        })?;
        self.node_definitions.insert(
            node_index,
            NodeDefinition::Task {
                worker_name: worker_name.to_owned(),
                task_name: task_name.to_owned(),
            },
        );
        self.build_inputs(inputs, node_index);
        self.build_outputs(outputs.into_keys(), node_index);

        Ok(())
    }

    fn build_inputs(
        &mut self,
        inputs: impl IntoIterator<Item = (String, ValueRef)>,
        node_index: NodeIndex,
    ) {
        let mut node_input_port_indices = HashMap::new();
        self.graph
            .inputs(node_index)
            .zip(inputs)
            .for_each(|(port_id, (port_name, source_ref))| {
                node_input_port_indices.insert(port_name.clone(), port_id);
                self.to_link.insert(port_id, source_ref);
                self.port_names.insert(port_id, port_name);
            });
        self.input_port_indices
            .insert(node_index, node_input_port_indices);
    }

    fn build_outputs(&mut self, outputs: impl IntoIterator<Item = String>, node_index: NodeIndex) {
        let mut node_output_port_indices = HashMap::new();
        self.graph
            .outputs(node_index)
            .zip(outputs)
            .for_each(|(port_id, port_name)| {
                node_output_port_indices.insert(port_name.clone(), port_id);
                self.port_names.insert(port_id, port_name);
            });
        self.output_port_indices
            .insert(node_index, node_output_port_indices);
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;

    #[test]
    fn empty_workflow_graph_roundtrip() -> miette::Result<()> {
        let workflow_graph = WorkflowGraph::new([]);

        let graph_bytes = serde_json::to_vec(&workflow_graph).into_diagnostic()?;
        let deserialized_graph = serde_json::from_slice(&graph_bytes).into_diagnostic()?;

        assert_eq!(workflow_graph, deserialized_graph);

        Ok(())
    }

    #[rstest]
    #[case(include_str!("../tests/cli/data/sample_graph"))]
    #[case(include_str!("../tests/cli/data/factorial"))]
    fn deserialize_graph(#[case] serialized_graph: &str) -> miette::Result<()> {
        let _graph: LegacyWorkflowGraph =
            serde_json::from_str(serialized_graph).into_diagnostic()?;

        Ok(())
    }

    #[rstest]
    #[case(include_str!("../tests/cli/data/sample_graph"))]
    #[case(include_str!("../tests/cli/data/factorial"))]
    fn convert_graph(#[case] serialized_graph: &str) -> miette::Result<()> {
        let graph: LegacyWorkflowGraph =
            serde_json::from_str(serialized_graph).into_diagnostic()?;
        dbg!(&graph);
        let converted = graph.to_workflow_graph()?;
        dbg!(&converted);

        let original: LegacyWorkflowGraph =
            serde_json::from_str(serialized_graph).into_diagnostic()?;

        assert_eq!(
            original.graph_output_idx,
            u32::try_from(converted.output_idx().index()).unwrap()
        );

        assert_eq!(original.nodes.len(), converted.node_definitions.len());
        for (idx, original_node) in original.nodes.iter().enumerate() {
            let converted_node = converted
                .node_definitions
                .get(&NodeIndex::new(idx))
                .unwrap();
            match (original_node, converted_node) {
                (
                    LegacyNodeDef::Func { function_name, .. },
                    NodeDefinition::Task {
                        worker_name,
                        task_name,
                    },
                ) => {
                    assert_eq!(function_name, &format!("{worker_name}.{task_name}"));
                }
                (
                    LegacyNodeDef::Const {
                        value: original_value,
                    },
                    NodeDefinition::Const {
                        value: converted_value,
                    },
                ) => {
                    assert_eq!(original_value, converted_value);
                }
                (
                    LegacyNodeDef::Input {
                        name: original_name,
                    },
                    NodeDefinition::Input {
                        name: converted_name,
                    },
                ) => {
                    assert_eq!(original_name, converted_name);
                }
                (LegacyNodeDef::Eval { .. }, NodeDefinition::Eval {})
                | (LegacyNodeDef::IfElse { .. }, NodeDefinition::IfElse {})
                | (LegacyNodeDef::Output { .. }, NodeDefinition::Output {})
                | (LegacyNodeDef::EagerIfElse { .. }, NodeDefinition::EagerIfElse {}) => {}
                _ => panic!("Node mismatch!: {original_node:?} != {converted_node:?}"),
            }
        }

        Ok(())
    }
}
