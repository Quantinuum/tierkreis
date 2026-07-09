

use crate::server::models::{NodeInputs, PyEdge, PyGraph, PyNode, function_name_from_def, node_status_from_state, node_type_from_def};
use crate::state::WorkflowRunState;
use crate::graph::{WorkflowGraph, NodeDefinition};
use crate::location::{Location};
use crate::server::AssetStorageRegistry;


/// Build a [`PyGraph`] for the given `workflow_graph`, reading live node states
/// from `run_state`.
///
/// `parent_location` is the prefix prepended to all node [`Location`]s in the
/// output.  Pass [`Location::root()`] for the top-level graph.
pub async fn build_py_graph<RS: WorkflowRunState>(
    workflow_graph: &WorkflowGraph,
    run_state: &RS,
    parent_location: &Location,
    //asset_registry: &AssetStorageRegistry,
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
        let outputs: Vec<String> = workflow_graph
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

            edges.push(PyEdge {
                from_node: from_location.to_string(),
                from_port: from_port_name.clone(),
                to_node: node_location_str.clone(),
                to_port: port_name.clone(),
                value: None, // TODO: load output value if available
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
                    // TODO load output value from run_state
                    None
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
            outputs,
            inputs,
            value,
            started_time: state.running_time.map_or_else(String::new, |t| t.to_rfc3339()),
            finished_time: state.complete_time.map_or_else(String::new, |t| t.to_rfc3339()),
        });
    }

    Ok(PyGraph { nodes, edges })
}
