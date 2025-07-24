use std::collections::{HashMap, HashSet};

use indexmap::{IndexMap, IndexSet};
use pyo3::{
    exceptions::PyValueError,
    prelude::*,
    types::{PyDict, PyType},
};
use pythonize::{depythonize, pythonize};
use serde::{Deserialize, Serialize};

/// Graph utilities
#[pymodule(submodule)]
pub mod graph {
    use pyo3::exceptions::PyIndexError;

    use crate::{
        identifiers::identifiers::{ExteriorOrValueRef, ExteriorRef, NodeIndex, PortID, ValueRef},
        location::location::{Loc, NodeStep},
        value::value::Value,
    };

    use super::*;

    /// Hack: workaround for https://github.com/PyO3/pyo3/issues/759
    #[pymodule_init]
    fn init(m: &Bound<'_, PyModule>) -> PyResult<()> {
        Python::attach(|py| {
            py.import("sys")?
                .getattr("modules")?
                .set_item("tierkreis_core._tierkreis_core.graph", m)
        })
    }

    /// new_eval_root is a utility function to create a new eval node that
    /// references the outer scope for its body.
    ///
    // Assumes the values have been written to storage using the same names
    #[pyfunction(signature = (inputs: "typing.Mapping[tierkreis_core.aliases.PortID, identifiers.ExteriorRef | identifiers.ValueRef]"))]
    pub fn new_eval_root(inputs: HashMap<PortID, ExteriorOrValueRef>) -> NodeDef {
        let inputs_refs = inputs
            .into_iter()
            .filter(|(port_id, _)| port_id != "body")
            .collect();
        NodeDef::Eval {
            body: ExteriorOrValueRef::Exterior(ExteriorRef("body".to_string())),
            inputs: inputs_refs,
        }
    }

    #[pyclass(eq, frozen)]
    #[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
    pub enum NodeDef {
        Func {
            name: String,
            inputs: IndexMap<PortID, ExteriorOrValueRef>,
        },
        Eval {
            body: ExteriorOrValueRef,
            inputs: IndexMap<PortID, ExteriorOrValueRef>,
        },
        Loop {
            name: Option<String>,
            body: ExteriorOrValueRef,
            inputs: IndexMap<PortID, ExteriorOrValueRef>,
            continue_port: PortID,
        },
        Map {
            body: ExteriorOrValueRef,
            inputs: IndexMap<PortID, ExteriorOrValueRef>,
        },
        Const {
            value: Value,
        },
        IfElse {
            pred: ValueRef,
            if_true: ValueRef,
            if_false: ValueRef,
        },
        EagerIfElse {
            pred: ValueRef,
            if_true: ValueRef,
            if_false: ValueRef,
        },
        Input {
            name: PortID,
        },
        Output {
            inputs: IndexMap<PortID, ValueRef>,
        },
    }

    impl std::fmt::Display for NodeDef {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                Self::Func { name, inputs } => write!(f, "Func('{}', {:?})", name, inputs),
                Self::Eval { inputs, .. } => write!(f, "Eval(..., {:?})", inputs),
                Self::Loop {
                    inputs,
                    continue_port,
                    ..
                } => write!(f, "Loop(..., {:?}, {})", inputs, continue_port),
                Self::Map { inputs, .. } => write!(f, "Map(..., {:?})", inputs),
                Self::Const { value } => write!(f, "Const({:?})", value),
                Self::IfElse {
                    pred,
                    if_true,
                    if_false,
                } => write!(f, "IfElse({}, {}, {})", pred, if_true, if_false),
                Self::EagerIfElse {
                    pred,
                    if_true,
                    if_false,
                } => write!(f, "EagerIfElse({}, {}, {})", pred, if_true, if_false),
                Self::Input { name } => write!(f, "Input('{}')", name),
                Self::Output { inputs } => write!(f, "Output({:?})", inputs),
            }
        }
    }

    #[pymethods]
    impl NodeDef {
        /// Get the `inputs` attribute for a Node. This is different to
        /// `in_edges` as it only considers the edges under the `inputs`
        /// attribute and not things like the body of Eval nodes which
        /// is considered a different kind of edge.
        #[getter]
        pub fn inputs(&self) -> IndexMap<PortID, ExteriorOrValueRef> {
            match self {
                Self::Eval { inputs, .. } => inputs.clone(),
                Self::Loop { inputs, .. } => inputs.clone(),
                Self::Map { inputs, .. } => inputs.clone(),
                Self::Func { inputs, .. } => inputs.clone(),
                Self::Const { .. } => IndexMap::new(),
                Self::IfElse { .. } => IndexMap::new(),
                Self::EagerIfElse { .. } => IndexMap::new(),
                Self::Input { .. } => IndexMap::new(),
                Self::Output { inputs, .. } => inputs
                    .iter()
                    .map(|x| (x.0.clone(), ExteriorOrValueRef::Value(x.1.clone())))
                    .collect(),
            }
        }

        /// Get the `in_edges` attribute for a Node. This is different to
        /// `inputs` as it also considers the edges connected to the body
        /// port of Eval nodes which does not come under the `inputs` attribute.
        #[getter]
        pub fn in_edges(&self) -> IndexMap<PortID, ExteriorOrValueRef> {
            match self {
                Self::Eval { inputs, body } => {
                    let mut inputs = inputs.clone();
                    inputs.insert("body".to_string(), body.clone());
                    inputs
                }
                Self::Loop { inputs, body, .. } => {
                    let mut inputs = inputs.clone();
                    inputs.insert("body".to_string(), body.clone());
                    inputs
                }
                Self::Map { inputs, body, .. } => {
                    let mut inputs = inputs.clone();
                    inputs.insert("body".to_string(), body.clone());
                    inputs
                }
                Self::Const { .. } => IndexMap::new(),
                Self::IfElse { pred, .. } => {
                    let mut inputs = IndexMap::new();
                    inputs.insert("pred".to_string(), ExteriorOrValueRef::Value(pred.clone()));
                    inputs
                }
                Self::EagerIfElse {
                    pred,
                    if_true,
                    if_false,
                } => {
                    let mut inputs = IndexMap::new();
                    inputs.insert("pred".to_string(), ExteriorOrValueRef::Value(pred.clone()));
                    inputs.insert(
                        "body_true".to_string(),
                        ExteriorOrValueRef::Value(if_true.clone()),
                    );
                    inputs.insert(
                        "body_false".to_string(),
                        ExteriorOrValueRef::Value(if_false.clone()),
                    );
                    inputs
                }
                Self::Func { inputs, .. } => inputs.clone(),
                Self::Input { .. } => IndexMap::new(),
                Self::Output { inputs, .. } => inputs
                    .iter()
                    .map(|x| (x.0.clone(), ExteriorOrValueRef::Value(x.1.clone())))
                    .collect(),
            }
        }

        pub fn model_dump_json(&self) -> String {
            serde_json::to_string(self).unwrap()
        }

        #[classmethod]
        pub fn model_load_json(_cls: &Bound<'_, PyType>, s: &str) -> Self {
            serde_json::from_str(s).unwrap()
        }

        pub fn __repr__(&self) -> String {
            format!("{}", self)
        }

        pub fn __str__(&self) -> String {
            format!("{}", self)
        }
    }

    #[pyclass]
    pub struct CurriedNodeIndex {
        node_index: NodeIndex,
    }

    #[pymethods]
    impl CurriedNodeIndex {
        fn __call__(&self, port: PortID) -> ValueRef {
            ValueRef(self.node_index.0, port)
        }
    }

    #[pyclass(eq)]
    #[derive(Default, Debug, Serialize, Deserialize, Clone, PartialEq)]
    pub struct GraphData {
        // First argument is the node definition, followed
        // by a set of which of the output ports have been
        // connected to other ports.
        nodes: Vec<(NodeDef, IndexMap<PortID, NodeIndex>)>,
        fixed_inputs: IndexMap<PortID, ()>,
        graph_inputs: IndexSet<PortID>,
        output_idx: Option<NodeIndex>,
    }

    #[pymethods]
    impl GraphData {
        #[new]
        pub fn new() -> Result<Self, pyo3::PyErr> {
            Ok(Self {
                ..Default::default()
            })
        }

        pub fn is_empty(&self) -> bool {
            self.nodes.is_empty()
        }

        pub fn get_nodedef(&self, node_idx: NodeIndex) -> PyResult<NodeDef> {
            self.nodes
                .get(node_idx.0)
                .map(|x| x.0.clone())
                .ok_or(PyIndexError::new_err(
                    "Node not found in graph with that index",
                ))
        }

        /// Retrieve the nodes in the graph as a list.
        #[getter]
        pub fn nodes(&self) -> Vec<NodeDef> {
            self.nodes.iter().map(|x| &x.0).cloned().collect()
        }

        #[getter]
        pub fn fixed_inputs(&self) -> IndexMap<PortID, ()> {
            self.fixed_inputs.clone()
        }

        /// Get the `NodeIndex` of the output node if it exists.
        ///
        /// For historical reasons this is a function rather
        /// than a getter.
        #[pyo3(signature = () -> "identifiers.NodeIndex | None")]
        pub fn output_idx(&self) -> Option<NodeIndex> {
            self.output_idx.clone()
        }

        pub fn input(&mut self, name: &str) -> ValueRef {
            let idx = self.add_node(NodeDef::Input {
                name: name.to_string(),
            });
            self.graph_inputs.insert(name.to_string());

            ValueRef(idx.0, name.to_string())
        }

        /// Add a Const node to the graph.
        #[pyo3(signature = (value: "tierkreis_core.aliases.Value"))]
        pub fn r#const(&mut self, value: Value) -> ValueRef {
            let idx = self.add_node(NodeDef::Const { value });

            ValueRef(idx.0, "value".to_string())
        }

        #[pyo3(signature = (
            function_name: "str | None",
            inputs: "typing.Mapping[tierkreis_core.aliases.PortID, identifiers.ValueRef]",
        ))]
        pub fn func(
            &mut self,
            function_name: &str,
            inputs: IndexMap<PortID, ValueRef>,
        ) -> CurriedNodeIndex {
            let node_index = self.add_node(NodeDef::Func {
                name: function_name.to_string(),
                inputs: inputs
                    .iter()
                    .map(|(port_id, edge_value_ref)| {
                        (
                            port_id.clone(),
                            ExteriorOrValueRef::Value(edge_value_ref.clone().clone()),
                        )
                    })
                    .collect(),
            });
            self.connect_inputs(node_index, &inputs);

            CurriedNodeIndex { node_index }
        }

        #[pyo3(signature = (
            graph: "identifiers.ExteriorRef | identifiers.ValueRef",
            inputs: "typing.Mapping[tierkreis_core.aliases.PortID, identifiers.ValueRef]",
        ))]
        pub fn eval(
            &mut self,
            graph: ExteriorOrValueRef,
            inputs: IndexMap<PortID, ValueRef>,
        ) -> CurriedNodeIndex {
            let node_index = self.add_node(NodeDef::Eval {
                body: graph,
                inputs: inputs
                    .iter()
                    .map(|(port_id, edge_value_ref)| {
                        (
                            port_id.clone(),
                            ExteriorOrValueRef::Value(edge_value_ref.clone().clone()),
                        )
                    })
                    .collect(),
            });
            self.connect_inputs(node_index, &inputs);

            CurriedNodeIndex { node_index }
        }

        #[pyo3(signature = (
            graph: "identifiers.ExteriorRef | identifiers.ValueRef",
            inputs: "typing.Mapping[tierkreis_core.aliases.PortID, identifiers.ValueRef]",
            continue_port: "tierkreis_core.aliases.PortID",
            name: "str | None" = None
        ))]
        pub fn r#loop(
            &mut self,
            graph: ExteriorOrValueRef,
            inputs: IndexMap<PortID, ValueRef>,
            continue_port: PortID,
            name: Option<String>,
        ) -> CurriedNodeIndex {
            let node_index = self.add_node(NodeDef::Loop {
                name,
                body: graph,
                inputs: inputs
                    .iter()
                    .map(|(port_id, edge_value_ref)| {
                        (
                            port_id.clone(),
                            ExteriorOrValueRef::Value(edge_value_ref.clone().clone()),
                        )
                    })
                    .collect(),
                continue_port,
            });
            self.connect_inputs(node_index, &inputs);

            CurriedNodeIndex { node_index }
        }

        #[pyo3(signature = (
            graph: "identifiers.ExteriorRef | identifiers.ValueRef",
            inputs: "typing.Mapping[tierkreis_core.aliases.PortID, identifiers.ValueRef]",
        ))]
        pub fn map(
            &mut self,
            graph: ExteriorOrValueRef,
            inputs: IndexMap<PortID, ValueRef>,
        ) -> CurriedNodeIndex {
            let node_index = self.add_node(NodeDef::Map {
                body: graph,
                inputs: inputs
                    .iter()
                    .map(|(port_id, edge_value_ref)| {
                        (
                            port_id.clone(),
                            ExteriorOrValueRef::Value(edge_value_ref.clone().clone()),
                        )
                    })
                    .collect(),
            });
            self.connect_inputs(node_index, &inputs);

            CurriedNodeIndex { node_index }
        }

        pub fn if_else(
            &mut self,
            pred: ValueRef,
            if_true: ValueRef,
            if_false: ValueRef,
        ) -> CurriedNodeIndex {
            let node_index = self.add_node(NodeDef::IfElse {
                pred: pred.clone(),
                if_true: if_true.clone(),
                if_false: if_false.clone(),
            });

            self.nodes[pred.0].1.insert(pred.1, node_index);
            self.nodes[if_true.0].1.insert(if_true.1, node_index);
            self.nodes[if_false.0].1.insert(if_false.1, node_index);

            CurriedNodeIndex { node_index }
        }

        pub fn eager_if_else(
            &mut self,
            pred: ValueRef,
            if_true: ValueRef,
            if_false: ValueRef,
        ) -> CurriedNodeIndex {
            let node_index = self.add_node(NodeDef::EagerIfElse {
                pred: pred.clone(),
                if_true: if_true.clone(),
                if_false: if_false.clone(),
            });

            self.nodes[pred.0].1.insert(pred.1, node_index);
            self.nodes[if_true.0].1.insert(if_true.1, node_index);
            self.nodes[if_false.0].1.insert(if_false.1, node_index);

            CurriedNodeIndex { node_index }
        }

        #[pyo3(signature = (
            inputs: "typing.Mapping[tierkreis_core.aliases.PortID, identifiers.ValueRef]",
        ) -> "None")]
        pub fn output(&mut self, inputs: IndexMap<PortID, ValueRef>) -> PyResult<()> {
            if self.output_idx.is_some() {
                return Err(PyValueError::new_err("Output already set"));
            }

            let node_index = self.add_node(NodeDef::Output {
                inputs: inputs.iter().map(|(k, v)| (k.clone(), v.clone())).collect(),
            });
            self.connect_inputs(node_index, &inputs);
            self.output_idx = Some(node_index);

            Ok(())
        }

        #[pyo3(signature = (provided_inputs: "typing.Set[tierkreis_core.aliases.PortID]") -> "set[tierkreis_core.aliases.PortID]")]
        pub fn remaining_inputs(&self, provided_inputs: HashSet<PortID>) -> HashSet<PortID> {
            let fixed_inputs: HashSet<PortID> = self.fixed_inputs.keys().cloned().collect();
            if fixed_inputs.intersection(&provided_inputs).next().is_some() {
                unimplemented!();
            }

            let actual_inputs: IndexSet<PortID> =
                fixed_inputs.union(&provided_inputs).cloned().collect();
            self.graph_inputs
                .difference(&actual_inputs)
                .cloned()
                .collect()
        }

        #[pyo3(signature = (node_index: "identifiers.NodeIndex") -> "dict[tierkreis_core.aliases.PortID, identifiers.NodeIndex] | None")]
        pub fn outputs(&self, node_index: NodeIndex) -> Option<IndexMap<PortID, NodeIndex>> {
            self.nodes.get(node_index.0).map(|(_, outputs)| {
                outputs
                    .iter()
                    .map(|(k, v)| (k.clone(), v.clone()))
                    .collect()
            })
        }

        #[pyo3(signature = () -> "dict[tierkreis_core.aliases.PortID, identifiers.NodeIndex] | None")]
        pub fn graph_outputs(&self) -> Option<IndexMap<PortID, NodeIndex>> {
            self.output_idx
                .and_then(|idx| self.nodes.get(idx.0))
                .and_then(|(node, _)| match node {
                    NodeDef::Output { inputs } => Some(
                        inputs
                            .iter()
                            .map(|(k, v)| (k.clone(), v.node_index()))
                            .collect(),
                    ),
                    _ => None,
                })
        }

        /// Query a NodeDescription from a Loc (which describes a location on the graph.)
        ///
        /// Useful for visualisation and debugging.
        pub fn query_node_description(&self, loc: Loc) -> PyResult<NodeDescription> {
            if self.is_empty() {
                return Err(PyValueError::new_err("Cannot query and empty graph"));
            }
            if loc.is_root_loc() {
                return Ok(NodeDescription {
                    definition: new_eval_root(HashMap::new()),
                    outputs: HashMap::new(),
                    outer_graph: Some(self.clone()),
                });
            }

            let (step, remaining) = loc.pop_first()?;
            let (nodedef, outputs) = match step {
                NodeStep::Node { node_index } => self
                    .nodes
                    .get(node_index.0)
                    .ok_or(PyIndexError::new_err("Node not found with that index"))?,
                NodeStep::Exterior {} => {
                    return Ok(NodeDescription {
                        definition: new_eval_root(HashMap::new()),
                        outputs: HashMap::new(),
                        outer_graph: Some(self.clone()),
                    })
                }
                _ => {
                    return Err(PyValueError::new_err(format!(
                        "Malformed Loc: First node is not a Node step: {}",
                        step
                    )))
                }
            };

            match nodedef {
                NodeDef::Eval {
                    body: ExteriorOrValueRef::Value(value_ref),
                    ..
                } => {
                    let connected_nodedef = self.get_nodedef(value_ref.node_index())?;
                    let const_graph = match connected_nodedef {
                        NodeDef::Const {
                            value: Value::Graph(graph),
                        } => graph,
                        _ => {
                            return Err(PyValueError::new_err(
                                "Const node connected to body port does not contain a graph",
                            ))
                        }
                    };
                    const_graph
                        .query_node_description(remaining)
                        .map(|mut desc| {
                            desc.outer_graph = Some(const_graph);
                            desc
                        })
                }
                NodeDef::Loop {
                    body: ExteriorOrValueRef::Value(value_ref),
                    inputs,
                    ..
                }
                | NodeDef::Map {
                    body: ExteriorOrValueRef::Value(value_ref),
                    inputs,
                    ..
                } => {
                    let connected_nodedef = self.get_nodedef(value_ref.node_index())?;
                    let const_graph = match connected_nodedef {
                        NodeDef::Const {
                            value: Value::Graph(graph),
                        } => graph,
                        _ => {
                            return Err(PyValueError::new_err(
                                "Const node connected to body port does not contain a graph",
                            ))
                        }
                    };
                    let (_, remaining) = remaining.pop_first()?;
                    if remaining.steps().len() < 2 {
                        return Ok(NodeDescription {
                            definition: NodeDef::Eval {
                                body: ExteriorOrValueRef::Exterior(ExteriorRef("body".to_string())),
                                inputs: inputs.clone(),
                            },
                            outputs: outputs
                                .iter()
                                .map(|(k, v)| (k.clone(), v.clone()))
                                .collect(),
                            outer_graph: Some(const_graph.clone()),
                        });
                    }
                    const_graph
                        .query_node_description(remaining)
                        .map(|mut desc| {
                            desc.outer_graph = Some(const_graph);
                            desc
                        })
                }
                _ => Ok(NodeDescription {
                    definition: nodedef.clone(),
                    outputs: outputs
                        .iter()
                        .map(|(k, v)| (k.clone(), v.clone()))
                        .collect(),
                    outer_graph: Some(self.clone()),
                }),
            }
        }

        pub fn to_dict<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyDict>> {
            Ok(pythonize(py, self)?.extract()?)
        }

        #[classmethod]
        pub fn from_dict(_cls: &Bound<'_, PyType>, obj: Bound<PyDict>) -> PyResult<Self> {
            Ok(depythonize(&obj)?)
        }

        pub fn model_dump<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyDict>> {
            self.to_dict(py)
        }

        #[classmethod]
        pub fn model_load(cls: &Bound<'_, PyType>, obj: Bound<PyDict>) -> PyResult<Self> {
            Self::from_dict(cls, obj)
        }

        pub fn model_dump_json(&self) -> String {
            serde_json::to_string(self).unwrap()
        }

        #[classmethod]
        pub fn model_load_json(_cls: &Bound<'_, PyType>, s: &str) -> Self {
            serde_json::from_str(s).unwrap()
        }
    }

    impl GraphData {
        fn connect_inputs(
            &mut self,
            new_node_index: NodeIndex,
            inputs: &IndexMap<PortID, ValueRef>,
        ) {
            for (_, ValueRef(idx, port)) in inputs {
                self.nodes[*idx].1.insert(port.clone(), new_node_index);
            }
        }

        fn add_node(&mut self, node: NodeDef) -> NodeIndex {
            // Nodes are 0-indexed so get the length before adding the new node.
            let node_index = self.nodes.len();
            self.nodes.push((node, IndexMap::new()));
            NodeIndex(node_index)
        }
    }

    /// An enriched version of NodeDef that contains the outputs
    /// and potentially the body of a sub-graph if available.
    ///
    /// Useful for visualisation and debugging.
    #[pyclass(eq)]
    #[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
    pub struct NodeDescription {
        #[pyo3(get)]
        pub definition: NodeDef,
        #[pyo3(get)]
        pub outputs: HashMap<PortID, NodeIndex>,
        // If the node is an eval, map or loop it may include an
        // embedded graph if the parent graph contains a const
        // node that is connected this node's body port.
        #[pyo3(get)]
        pub outer_graph: Option<GraphData>,
    }

    #[pymethods]
    impl NodeDescription {
        #[new]
        #[pyo3(signature = (
            definition: "graph.NodeDef",
            outputs: "typing.Mapping[tierkreis_core.aliases.PortID, identifiers.NodeIndex]"  = IndexMap::new(),
            outer_graph: "graph.GraphData | None" = None,
        ))]
        pub fn new(
            definition: NodeDef,
            outputs: IndexMap<PortID, NodeIndex>,
            outer_graph: Option<GraphData>,
        ) -> PyResult<Self> {
            Ok(Self {
                definition,
                outputs: outputs.into_iter().collect(),
                outer_graph,
            })
        }

        pub fn __repr__(&self) -> String {
            format!("{}", self)
        }

        pub fn __str__(&self) -> String {
            format!("{}", self)
        }

        pub fn to_dict<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyDict>> {
            Ok(pythonize(py, self)?.extract()?)
        }

        #[classmethod]
        pub fn from_dict(_cls: &Bound<'_, PyType>, obj: Bound<PyDict>) -> PyResult<Self> {
            Ok(depythonize(&obj)?)
        }

        pub fn model_dump<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyDict>> {
            self.to_dict(py)
        }

        #[classmethod]
        pub fn model_load(cls: &Bound<'_, PyType>, obj: Bound<PyDict>) -> PyResult<Self> {
            Self::from_dict(cls, obj)
        }

        pub fn model_dump_json(&self) -> String {
            serde_json::to_string(self).unwrap()
        }

        #[classmethod]
        pub fn model_load_json(_cls: &Bound<'_, PyType>, s: &str) -> Self {
            serde_json::from_str(s).unwrap()
        }
    }

    impl std::fmt::Display for NodeDescription {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(
                f,
                "NodeDescription({}, {:?}, {:?})",
                self.definition, self.outputs, self.outer_graph
            )
        }
    }
}
