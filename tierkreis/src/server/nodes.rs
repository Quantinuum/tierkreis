use std::collections::HashMap;

use miette::Context;

use crate::asset_storage::{load_asset, load_assets};
use crate::graph::{LegacyWorkflowGraph, NodeDefinition, WorkflowGraph};
use crate::location::{Location, LocationComponent};
use crate::server::AssetStorageRegistry;
use crate::server::models::{
    NodeInputs, NodeStatus, PyEdge, PyGraph, PyNode, node_status_from_state,
};
use crate::state::{WorkflowRunState, interface::NodeState};
/// The result of resolving a location to a graph view.
pub enum GraphLoadResult {
    /// Show the nodes of this workflow graph (standard eval/top-level view).
    Eval {
        graph: WorkflowGraph,
        prefix: Location,
    },
    /// Show loop iteration placeholder nodes for this loop node.
    LoopIterations {
        loop_node_location: Location,
        subgraph: WorkflowGraph,
    },
    /// Show map element placeholder nodes for this map node.
    MapIterations {
        map_node_location: Location,
        subgraph: WorkflowGraph,
    },
}

/// Attempt to load a JSON-serialized value for a given port name.
/// Returns an error on missing/incomplete data or parse failure.
///
/// # Errors
///
///  If the node has no outputs, if the output port is missing, if the storage is not found, or if the value cannot be deserialized.
pub async fn try_load_output_value(
    port_name: &str,
    node_state: &NodeState,
    asset_registry: &AssetStorageRegistry,
) -> miette::Result<serde_json::Value> {
    tracing::info!(
        "Trying to load output value for port {port_name} from node state {node_state:?}"
    );
    let outputs = node_state
        .outputs
        .as_ref()
        .ok_or_else(|| miette::miette!("Node has no outputs"))?;
    let bytes = load_asset(asset_registry, outputs, port_name).await?;
    serde_json::from_slice::<serde_json::Value>(&bytes).map_err(|e| miette::miette!(e.to_string()))
}

/// Attempt to load all outputs for a given node state, returning a map of port name to value.
///
/// # Errors
///
/// If the node has no outputs, if any output port is missing, if the storage is not found, or if any value cannot be deserialized.
pub async fn try_load_outputs(
    node_state: &NodeState,
    asset_registry: &AssetStorageRegistry,
) -> miette::Result<HashMap<String, serde_json::Value>> {
    let Some(outputs) = &node_state.outputs else {
        return Ok(HashMap::new());
    };
    tracing::info!("Trying to load output values from node state {node_state:?}");
    let assets = load_assets(asset_registry, outputs)
        .await?
        .into_iter()
        .map(|(port_name, bytes)| {
            let value = serde_json::from_slice::<serde_json::Value>(&bytes)
                .map_err(|e| miette::miette!(e.to_string()))?;
            Ok((port_name, value))
        })
        .collect::<miette::Result<HashMap<String, serde_json::Value>>>()?;

    Ok(assets)
}

/// Build a [`PyGraph`] showing loop iteration placeholder nodes for a Loop node.
///
/// Each placeholder represents one loop iteration (`L0`, `L1`, …).  All iterations
/// before the current one are marked `Finished`; the current one gets its live
/// status derived from the loop node's own state.  Edges are emitted from
/// iteration `i` to `i+1` for every output port.
///
/// The iteration count is read from [`NodeState::loop_index`] on the loop node
/// itself — virtual `Li` locations do not carry their own `NodeState`.
///
/// # Errors
///
/// If the loop node state cannot be read.
pub async fn build_loop_py_graph(
    loop_node_location: &Location,
    subgraph: &WorkflowGraph,
    run_state: &dyn WorkflowRunState,
    asset_registry: &AssetStorageRegistry,
) -> miette::Result<PyGraph> {
    // Output port names come from the input ports of the subgraph's Output node.
    let output_names: Vec<String> = subgraph
        .input_names(subgraph.output_idx())
        .map(|it| it.cloned().collect())
        .unwrap_or_default();

    // The loop node carries loop_index = Some(n) meaning "iteration n is current".
    // Virtual Li locations have no NodeState of their own.
    let loop_state = run_state.read(loop_node_location).await.unwrap_or_default();
    let current_idx = loop_state.loop_index.unwrap_or(0);

    // Derive the status of the current (last) iteration from the loop node itself.
    let current_status = if loop_state.complete_time.is_some() {
        NodeStatus::Finished
    } else if loop_state.error_time.is_some() || loop_state.cancelled_time.is_some() {
        NodeStatus::Error
    } else if loop_state.running_time.is_some() {
        NodeStatus::Running
    } else {
        NodeStatus::NotStarted
    };

    let mut nodes = Vec::new();
    let mut edges = Vec::new();

    for i in 0..=current_idx {
        let iter_loc = loop_node_location.with_loop_index(i);
        // All iterations before the current one are finished; the current one uses the
        // live status derived above.  Virtual Li locations have no NodeState.
        let status = if i < current_idx {
            NodeStatus::Finished
        } else {
            current_status.clone()
        };
        nodes.push(PyNode {
            id: iter_loc.to_string(),
            status,
            function_name: format!("L{i}"),
            node_type: NodeDefinition::Eval {}.node_type(),
            node_location: iter_loc.to_string(),
            outputs: output_names.clone(),
            inputs: Vec::new(),
            value: None,
            // Timestamps are not tracked per-iteration; use the loop node's times
            // for the current iteration only.
            started_time: if i == current_idx {
                loop_state
                    .running_time
                    .map_or_else(String::new, |t| t.to_rfc3339())
            } else {
                String::new()
            },
            finished_time: if i == current_idx && loop_state.complete_time.is_some() {
                loop_state
                    .complete_time
                    .map_or_else(String::new, |t| t.to_rfc3339())
            } else {
                String::new()
            },
        });
    }

    // Edges: connect each completed iteration's outputs to the next iteration's inputs.
    // Edge values are stored on the loop subgraph output node inside each iteration.
    for port_name in &output_names {
        for i in 0..current_idx {
            let from_loc = loop_node_location.with_loop_index(i);
            let to_loc = loop_node_location.with_loop_index(i + 1);
            // The loop subgraph output node is at from_loc.with_node(subgraph.output_idx()).
            let output_node_loc = from_loc.with_node(subgraph.output_idx());
            let output_node_state = run_state.read(&output_node_loc).await.unwrap_or_default();
            let value = try_load_output_value(port_name, &output_node_state, asset_registry)
                .await
                .ok()
                .map(|v| v.to_string());
            edges.push(PyEdge {
                from_node: from_loc.to_string(),
                from_port: port_name.clone(),
                to_node: to_loc.to_string(),
                to_port: port_name.clone(),
                value,
                conditional: false,
            });
        }
    }

    Ok(PyGraph { nodes, edges })
}

/// Build a [`PyGraph`] showing map element placeholder nodes for a Map node.
///
/// Each placeholder represents one map element (`M0`, `M1`, …).  The total element
/// count and per-element completion are taken from the `map_completed` bit-vector
/// stored on the map node's state.  Virtual `Mi` locations do not carry their own
/// `NodeState`; status is derived from the `map_completed` bits instead.
///
/// # Errors
///
/// If the map node state cannot be read.
pub async fn build_map_py_graph(
    map_node_location: &Location,
    subgraph: &WorkflowGraph,
    run_state: &dyn WorkflowRunState,
    asset_registry: &AssetStorageRegistry,
) -> miette::Result<PyGraph> {
    let _ = asset_registry; // no edges between map elements

    // Output port names come from the input ports of the subgraph's Output node.
    let output_names: Vec<String> = subgraph
        .input_names(subgraph.output_idx())
        .map(|it| it.cloned().collect())
        .unwrap_or_default();

    // The map node state carries a bit-vector: set by SetRunningMap / MapElemComplete.
    // Virtual Mi locations have no NodeState of their own.
    let map_state = run_state.read(map_node_location).await.unwrap_or_default();
    let Some(ref map_completed) = map_state.map_completed else {
        // Map hasn't started ye
        return Ok(PyGraph {
            nodes: Vec::new(),
            edges: Vec::new(),
        });
    };
    let num_elements = map_completed.len();
    let map_done = map_state.complete_time.is_some();

    let mut nodes = Vec::new();
    for i in 0..num_elements {
        let elem_loc = map_node_location.with_map_index(i);
        // Status is derived from the bit-vector, not from a virtual Mi NodeState.
        let elem_done = map_completed[i] || map_done;
        let status = if elem_done {
            NodeStatus::Finished
        } else if map_state.error_time.is_some() {
            NodeStatus::Error
        } else {
            NodeStatus::Running
        };
        nodes.push(PyNode {
            id: elem_loc.to_string(),
            status,
            function_name: format!("M{i}"),
            node_type: NodeDefinition::Eval {}.node_type(),
            node_location: elem_loc.to_string(),
            outputs: output_names.clone(),
            inputs: Vec::new(),
            value: None,
            started_time: map_state
                .running_time
                .map_or_else(String::new, |t| t.to_rfc3339()),
            finished_time: if elem_done {
                map_state
                    .complete_time
                    .map_or_else(String::new, |t| t.to_rfc3339())
            } else {
                String::new()
            },
        });
    }

    Ok(PyGraph {
        nodes,
        edges: Vec::new(),
    })
}

/// Build a [`PyGraph`] for the given `workflow_graph`, reading live node states
/// from `run_state`.
///
/// `parent_location` is the prefix prepended to all node [`Location`]s in the
/// output.  Pass [`Location::root()`] for the top-level graph.
///
/// # Errors
///
/// If the node state cannot be read or if the graph cannot be built.
pub async fn build_py_graph(
    workflow_graph: &WorkflowGraph,
    run_state: &dyn WorkflowRunState,
    parent_location: &Location,
    asset_registry: &AssetStorageRegistry,
) -> miette::Result<PyGraph> {
    let mut nodes = Vec::new();
    let mut edges = Vec::new();

    for node_idx in workflow_graph.node_ids() {
        let node_location = parent_location.with_node(node_idx);
        let node_location_str = node_location.to_string();

        let def = workflow_graph
            .node_definition(node_idx)
            .ok_or_else(|| miette::miette!("Node definition missing for {node_idx:?}"))?;

        let node_type = def.node_type();
        let function_name = def.function_name();
        let state = run_state.read(&node_location).await?;
        let status = node_status_from_state(&state);

        // Collect output port names.
        let output_names: Vec<String> = workflow_graph
            .output_names(node_idx)
            .map(|it| it.cloned().collect::<Vec<_>>())
            .unwrap_or_default();

        // Collect incoming connections for all input ports.
        let input_links: Vec<_> = workflow_graph.input_links(node_idx).collect();
        let mut inputs = Vec::new();
        for (input_sub, output_sub) in &input_links {
            let input_port: portgraph::PortIndex = (*input_sub).into();
            let output_port: portgraph::PortIndex = (*output_sub).into();

            let port_name = workflow_graph.get_port_name(input_port)?;
            let from_port_name = workflow_graph.get_port_name(output_port)?;
            let from_node_idx = workflow_graph.port_node(*output_sub)?;
            let from_location = parent_location.with_node(from_node_idx);

            inputs.push(NodeInputs {
                port: port_name.clone(),
                from_node: from_location.to_string(),
                from_port: from_port_name.clone(),
            });

            // Conditional flag for IfElse branches.
            let conditional = matches!(
                def,
                NodeDefinition::IfElse {} | NodeDefinition::EagerIfElse {}
            ) && (port_name == "if_true" || port_name == "if_false");
            tracing::info!(
                "Adding edge from {}.{} to {}.{})",
                from_location,
                from_port_name,
                node_location_str,
                port_name
            );
            let node_state = run_state.read(&from_location).await.unwrap_or_default();

            edges.push(PyEdge {
                from_node: from_location.to_string(),
                from_port: from_port_name.clone(),
                to_node: node_location_str.clone(),
                to_port: port_name.clone(),
                value: try_load_output_value(from_port_name, &node_state, asset_registry)
                    .await
                    .ok()
                    .map(|v| v.to_string()),
                conditional,
            });
        }

        // Determine node value string.
        let value = match def {
            NodeDefinition::Const { value } => {
                Some(serde_json::to_string(value).unwrap_or_default())
            }
            NodeDefinition::Input { name } => Some(name.clone()),
            NodeDefinition::Output {} if input_links.len() == 1 => {
                // Show the single input value if available.
                let (_, output_sub) = &input_links[0];
                let output_port: portgraph::PortIndex = (*output_sub).into();
                let from_port_name = workflow_graph.get_port_name(output_port).ok().cloned();
                let from_node_idx = workflow_graph.port_node(*output_sub).ok();
                if let (Some(port), Some(src_node)) = (from_port_name, from_node_idx) {
                    let src_loc = parent_location.with_node(src_node);
                    let src_state = run_state.read(&src_loc).await.unwrap_or_default();
                    try_load_output_value(&port, &src_state, asset_registry)
                        .await
                        .ok()
                        .map(|v| v.to_string())
                } else {
                    None
                }
            }
            _ => None,
        };

        nodes.push(PyNode {
            id: node_location_str.clone(),
            status,
            function_name,
            node_type,
            node_location: node_location_str,
            outputs: output_names,
            inputs,
            value,
            started_time: state
                .running_time
                .map_or_else(String::new, |t| t.to_rfc3339()),
            finished_time: state
                .complete_time
                .map_or_else(String::new, |t| t.to_rfc3339()),
        });
    }

    Ok(PyGraph { nodes, edges })
}

/// Load a subgraph from a Const node that contains a serialized `WorkflowGraph`.
fn load_subgraph_from_const_node(
    workflow_graph: &WorkflowGraph,
    node_index: portgraph::NodeIndex,
) -> miette::Result<WorkflowGraph> {
    tracing::info!("Loading subgraph from Const node {node_index:?}");
    let (source_node, _source_port) = workflow_graph
        .connected_input_by_port_name(node_index, "graph")
        .wrap_err_with(|| {
            format!("No connected input by port name 'graph' for node {node_index:?}")
        })?;
    let source_def = workflow_graph
        .node_definition(source_node)
        .ok_or_else(|| miette::miette!("Node definition missing for {source_node:?}"))?;
    if let NodeDefinition::Const { value } = source_def {
        if let Ok(val) = serde_json::from_value::<WorkflowGraph>(value.clone()) {
            tracing::info!("Loaded subgraph from Const node {node_index:?}: {:?}", val);
            Ok(val)
        } else {
            let legacy_val = serde_json::from_value::<LegacyWorkflowGraph>(value.clone())
                .map_err(|_| miette::miette!("Fallback Failed"))?;
            Ok(legacy_val.to_workflow_graph()?)
        }
    } else {
        Err(miette::miette!("Node {node_index:?} is not a Const node"))
    }
}

/// Resolve the graph view for a given `location_str`.
/// # Errors
///
/// If the location string is invalid, if a node in the path is not found, or if a node
/// in the path is not an Eval, Loop, or Map node.
pub async fn load_graph(
    top_level_graph: &WorkflowGraph,
    location_str: &str,
) -> miette::Result<GraphLoadResult> {
    if location_str.is_empty() || location_str == "-" {
        return Ok(GraphLoadResult::Eval {
            graph: top_level_graph.clone(),
            prefix: Location::root(),
        });
    }

    let loc = Location::new(location_str)?;
    let components: Vec<&LocationComponent> = loc.components().collect();
    let n = components.len();

    // Walk the location path component by component, descending into subgraphs.
    let mut current_graph = top_level_graph.clone();
    let mut prefix = Location::root();

    for (i, component) in components.iter().enumerate() {
        match component {
            LocationComponent::Node { node } => {
                let node_loc = prefix.with_node(*node);
                let def = current_graph
                    .node_definition(*node)
                    .ok_or_else(|| miette::miette!("Node {node:?} not found in graph"))?;

                match def {
                    NodeDefinition::Eval {} => {
                        prefix = node_loc;
                        current_graph = load_subgraph_from_const_node(&current_graph, *node)?;
                    }
                    NodeDefinition::Loop {} => {
                        // If the next component is a LoopIndex, descend and continue walking.
                        // Otherwise this is the terminal location: return iteration placeholders.
                        let next_is_loop_index = i + 1 < n
                            && matches!(components[i + 1], LocationComponent::LoopIndex { .. });
                        if next_is_loop_index {
                            prefix = node_loc;
                            current_graph = load_subgraph_from_const_node(&current_graph, *node)?;
                        } else {
                            let subgraph = load_subgraph_from_const_node(&current_graph, *node)?;
                            return Ok(GraphLoadResult::LoopIterations {
                                loop_node_location: node_loc,
                                subgraph,
                            });
                        }
                    }
                    NodeDefinition::Map { .. } => {
                        // If the next component is a MapIndex, descend and continue walking.
                        // Otherwise this is the terminal location: return iteration placeholders.
                        let next_is_map_index = i + 1 < n
                            && matches!(components[i + 1], LocationComponent::MapIndex { .. });
                        if next_is_map_index {
                            prefix = node_loc;
                            current_graph = load_subgraph_from_const_node(&current_graph, *node)?;
                        } else {
                            let subgraph = load_subgraph_from_const_node(&current_graph, *node)?;
                            return Ok(GraphLoadResult::MapIterations {
                                map_node_location: node_loc,
                                subgraph,
                            });
                        }
                    }
                    _ => {
                        return Err(miette::miette!(
                            "Node {node:?} at {node_loc} is not an Eval, Loop, or Map node"
                        ));
                    }
                }
            }
            LocationComponent::LoopIndex { index } => {
                prefix = prefix.with_loop_index(*index);
            }
            LocationComponent::MapIndex { index } => {
                prefix = prefix.with_map_index(*index as usize);
            }
        }
    }
    Ok(GraphLoadResult::Eval {
        graph: current_graph,
        prefix,
    })
}
