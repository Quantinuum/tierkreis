

use crate::server::models::{NodeInputs, PyEdge, PyGraph, PyNode, function_name_from_def, node_status_from_state, node_type_from_def};
use crate::state::{WorkflowRunState, interface::NodeState};
use crate::graph::{WorkflowGraph, NodeDefinition};
use crate::location::{Location};
use crate::server::AssetStorageRegistry;


/// Attempt to load a JSON-serialized value for a given port name.
/// Returns `None` on any error (missing asset, not yet complete, etc.).
async fn try_load_output_value(
    port_name: &str,
    node_state: &NodeState,
    asset_registry: &AssetStorageRegistry,
) -> Option<String> {
    let outputs = node_state.outputs.as_ref()?;
    tracing::info!("Trying to load output value for port {port_name} with outputs: {:?}", outputs);
    let asset_spec = outputs.get(port_name)?;
    let bytes = asset_registry
        .read()
        .ok()?
        .get(&asset_spec.storage_name)?
        .load(&asset_spec.asset_key)
        .ok()?;
    // Try to pretty-print as JSON, fall back to raw UTF-8.
    if let Ok(json_val) = serde_json::from_slice::<serde_json::Value>(&bytes) {
        Some(serde_json::to_string(&json_val).unwrap_or_default())
    } else {
        String::from_utf8(bytes).ok()
    }
}

// async fn try_load_value(
    
// )


/// Build a [`PyGraph`] for the given `workflow_graph`, reading live node states
/// from `run_state`.
///
/// `parent_location` is the prefix prepended to all node [`Location`]s in the
/// output.  Pass [`Location::root()`] for the top-level graph.
pub async fn build_py_graph<RS: WorkflowRunState>(
    workflow_graph: &WorkflowGraph,
    run_state: &RS,
    parent_location: &Location,
    asset_registry: &AssetStorageRegistry,
) -> miette::Result<PyGraph> {
    let mut nodes = Vec::new();
    let mut edges = Vec::new();

    for node_idx in workflow_graph.node_ids() {
        let node_location = parent_location.with_node(node_idx);
        let node_location_str = node_location.to_string();

        tracing::info!("Parsing node {:?} at {}", node_idx, node_location_str);
        let def = workflow_graph
            .node_definition(node_idx)
            .ok_or_else(|| miette::miette!("Node definition missing for {node_idx:?}"))?;

        let node_type = node_type_from_def(def);
        let function_name = function_name_from_def(def);
        let state = run_state.read(&node_location).await?;
        let status = node_status_from_state(&state);

        tracing::info!("Found node {:?} {:?} {:?}", node_type, function_name, status);
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
            let conditional =
                matches!(def, NodeDefinition::IfElse {} | NodeDefinition::EagerIfElse {})
                    && (port_name == "if_true" || port_name == "if_false");
            tracing::info!("Adding edge from {}.{} to {}.{})", from_location, from_port_name, node_location_str, port_name);
            let node_state = run_state.read(&from_location).await.unwrap_or_default();

            edges.push(PyEdge {
                from_node: from_location.to_string(),
                from_port: from_port_name.clone(),
                to_node: node_location_str.clone(),
                to_port: port_name.clone(),
                value: try_load_output_value(from_port_name, &node_state, asset_registry).await,
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
                let from_port_name = workflow_graph
                    .get_port_name(output_port)
                    .ok()
                    .map(Clone::clone);
                let from_node_idx = workflow_graph.port_node(*output_sub).ok();
                if let (Some(port), Some(src_node)) = (from_port_name, from_node_idx) {
                    let src_loc = parent_location.with_node(src_node);
                    let src_state = run_state.read(&src_loc).await.unwrap_or_default();
                    try_load_output_value(&port, &src_state, asset_registry).await
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
            started_time: state.running_time.map_or_else(String::new, |t| t.to_rfc3339()),
            finished_time: state.complete_time.map_or_else(String::new, |t| t.to_rfc3339()),
        });
    }

    Ok(PyGraph { nodes, edges })
}
