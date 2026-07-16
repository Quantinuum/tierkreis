use std::collections::HashMap;

use crate::graph::{LegacyWorkflowGraph, NodeDefinition, WorkflowGraph};
use crate::location::Location;
use crate::server::AssetStorageRegistry;
use crate::server::models::{
    NodeInputs, PyEdge, PyGraph, PyNode, function_name_from_def, node_status_from_state,
    node_type_from_def,
};
use crate::state::{WorkflowRunState, interface::NodeState};

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
    let outputs = node_state
        .outputs
        .as_ref()
        .ok_or_else(|| miette::miette!("Node has no outputs"))?;
    let asset_spec = outputs
        .get(port_name)
        .ok_or_else(|| miette::miette!("Missing output port '{port_name}'"))?;
    let registry = asset_registry.read().await;
    let bytes = registry
        .get(&asset_spec.storage_name)
        .ok_or_else(|| miette::miette!("Storage '{}' not found", asset_spec.storage_name))?
        .load(&asset_spec.asset_key)
        .await?;
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
    let mut loaded = HashMap::new();
    for name in outputs.keys() {
        if let Ok(val) = try_load_output_value(name, node_state, asset_registry).await {
            loaded.insert(name.clone(), val);
        }
    }
    Ok(loaded)
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

        let node_type = node_type_from_def(def);
        let function_name = function_name_from_def(def);
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
    let Some((source_node, _source_port)) = workflow_graph
        .connected_input_by_port_name(node_index, "graph")
        .ok()
    else {
        return Err(miette::miette!(
            "No connected input by port name 'graph' for node {node_index:?}"
        ));
    };
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

/// Resolve which `WorkflowGraph` and `Location` prefix to use for a given `location_str`.
///
/// # Errors
///
/// If the location string is invalid, if a node in the path is not found, or if a node in the path is not an Eval, Loop, or Map node.
pub async fn load_graph(
    top_level_graph: &WorkflowGraph,
    location_str: &str,
) -> miette::Result<(WorkflowGraph, Location)> {
    if location_str.is_empty() || location_str == "-" {
        return Ok((top_level_graph.clone(), Location::root()));
    }

    let loc = Location::new(location_str)?;

    // Walk the location path component by component, descending into subgraphs.
    let mut current_graph = top_level_graph.clone();
    let mut prefix = Location::root();

    for component in loc.components() {
        use crate::location::LocationComponent;
        match component {
            LocationComponent::Node { node } => {
                let node_loc = prefix.with_node(*node);
                let def = current_graph
                    .node_definition(*node)
                    .ok_or_else(|| miette::miette!("Node {node:?} not found in graph"))?;

                match def {
                    NodeDefinition::Eval {}
                    | NodeDefinition::Loop {}
                    | NodeDefinition::Map { .. } => {
                        prefix = node_loc;
                        current_graph = load_subgraph_from_const_node(&current_graph, *node)?;
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
    Ok((current_graph, prefix))
}
