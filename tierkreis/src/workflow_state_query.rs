//! Utilities for loading and traversing workflow graphs using persisted runtime state.

use std::{collections::HashMap, collections::HashSet, sync::Arc};

use futures::{FutureExt, future::BoxFuture};
use miette::{Context, IntoDiagnostic, miette};
use portgraph::NodeIndex;

use crate::{
    asset_storage::{AssetSpec, AssetStorageRegistry, load_asset},
    graph::{LegacyWorkflowGraph, NodeDefinition, WorkflowGraph},
    location::{Location, LocationComponent},
    state::{WorkflowRunState, interface::NodeState},
};

pub(crate) async fn collect_node_states(
    workflow_run_state: &Arc<dyn WorkflowRunState>,
    workflow_graph: &WorkflowGraph,
    parent_loc: &Location,
) -> miette::Result<HashMap<Location, NodeState>> {
    workflow_run_state
        .read_many(&mut workflow_graph.node_ids().map(|n| parent_loc.with_node(n)))
        .await
}

/// Load a subgraph from the `graph` input, accepting current and legacy formats.
///
/// # Errors
///
/// Returns Err if the graph asset cannot be loaded or decoded.
#[tracing::instrument(skip_all, err)]
pub async fn load_subgraph<S: ::std::hash::BuildHasher>(
    asset_storage_registry: &AssetStorageRegistry,
    inputs: &HashMap<String, AssetSpec, S>,
) -> miette::Result<WorkflowGraph> {
    let subgraph_bytes = load_asset(asset_storage_registry, inputs, "graph").await?;
    let subgraph_res: Result<WorkflowGraph, serde_json::Error> =
        serde_json::from_slice(&subgraph_bytes);

    let subgraph = match subgraph_res {
        Ok(subgraph) => subgraph,
        Err(_err) => {
            let legacy: LegacyWorkflowGraph =
                serde_json::from_slice(&subgraph_bytes).into_diagnostic()?;
            legacy.to_workflow_graph()?
        }
    };

    Ok(subgraph)
}

/// Resolve the graph containing the terminal node of `loc`, descending into subgraphs.
///
/// # Errors
///
/// Returns Err if the location is malformed or a subgraph cannot be loaded.
pub async fn resolve_location(
    asset_storage_registry: &AssetStorageRegistry,
    workflow_run_state: &Arc<dyn WorkflowRunState>,
    root_graph: &Arc<WorkflowGraph>,
    loc: &Location,
) -> miette::Result<(WorkflowGraph, Location, NodeIndex)> {
    let mut current_graph = root_graph.as_ref().clone();
    let mut parent_loc = Location::root();
    let mut current_node: Option<NodeIndex> = None;

    let node_states = workflow_run_state
        .read_many(
            &mut current_graph
                .node_ids()
                .map(|node| parent_loc.with_node(node)),
        )
        .await?;

    for component in loc.components() {
        match component {
            LocationComponent::Node { node } => {
                if let Some(prev_node) = current_node
                    && matches!(
                        current_graph.node_definition(prev_node),
                        Some(NodeDefinition::Eval {})
                    )
                {
                    let inputs =
                        collect_inputs(&current_graph, &node_states, &parent_loc, prev_node)?;
                    if inputs.contains_key("graph") {
                        current_graph = load_subgraph(asset_storage_registry, &inputs).await?;
                        parent_loc = parent_loc.with_node(prev_node);
                    }
                }
                current_node = Some(*node);
            }
            LocationComponent::LoopIndex { index } => {
                let node = current_node.ok_or_else(|| {
                    miette!("Malformed location {loc}: loop iteration without an enclosing node")
                })?;
                let inputs = collect_inputs(&current_graph, &node_states, &parent_loc, node)?;
                current_graph = load_subgraph(asset_storage_registry, &inputs).await?;
                parent_loc = parent_loc.with_node(node).with_loop_index(*index);
                current_node = None;
            }
            LocationComponent::MapIndex { index } => {
                let node = current_node.ok_or_else(|| {
                    miette!("Malformed location {loc}: map iteration without an enclosing node")
                })?;
                let inputs = collect_inputs(&current_graph, &node_states, &parent_loc, node)?;
                current_graph = load_subgraph(asset_storage_registry, &inputs).await?;
                parent_loc = parent_loc
                    .with_node(node)
                    .with_map_index(usize::try_from(*index).into_diagnostic()?);
                current_node = None;
            }
        }
    }

    let node = current_node.ok_or_else(|| miette!("Location {loc} does not refer to a node"))?;
    Ok((current_graph, parent_loc, node))
}

/// Compute transitive consumers of a location, including later enclosing loop iterations.
///
/// # Errors
///
/// Returns Err if a location cannot be resolved or its state cannot be read.
pub fn dependents<'a>(
    asset_storage_registry: &'a AssetStorageRegistry,
    workflow_run_state: &'a Arc<dyn WorkflowRunState>,
    root_graph: &'a Arc<WorkflowGraph>,
    loc: &'a Location,
) -> BoxFuture<'a, miette::Result<HashSet<Location>>> {
    async move {
        let mut descendants = HashSet::new();
        let Some((parent, last)) = loc.split_last() else {
            return Ok(descendants);
        };

        match last {
            LocationComponent::Node { .. } => {
                let (graph, graph_prefix, node) =
                    resolve_location(asset_storage_registry, workflow_run_state, root_graph, loc)
                        .await?;

                if matches!(graph.node_definition(node), Some(NodeDefinition::Output {})) {
                    descendants.extend(
                        dependents(
                            asset_storage_registry,
                            workflow_run_state,
                            root_graph,
                            &parent,
                        )
                        .await?,
                    );
                }

                for child in graph.output_neighbours(node) {
                    let child_loc = graph_prefix.with_node(child);
                    if descendants.insert(child_loc.clone()) {
                        descendants.extend(
                            dependents(
                                asset_storage_registry,
                                workflow_run_state,
                                root_graph,
                                &child_loc,
                            )
                            .await?,
                        );
                    }
                }
            }
            LocationComponent::LoopIndex { index } => {
                let latest = workflow_run_state
                    .read(&parent)
                    .await?
                    .loop_index
                    .unwrap_or(0);
                for iteration in (index + 1)..=latest {
                    descendants.insert(parent.with_loop_index(iteration));
                }
                descendants.extend(
                    dependents(
                        asset_storage_registry,
                        workflow_run_state,
                        root_graph,
                        &parent,
                    )
                    .await?,
                );
            }
            LocationComponent::MapIndex { .. } => {
                descendants.extend(
                    dependents(
                        asset_storage_registry,
                        workflow_run_state,
                        root_graph,
                        &parent,
                    )
                    .await?,
                );
            }
        }

        Ok(descendants)
    }
    .boxed()
}

/// Find the `graph` input to a Node at a specified Location and return it.
pub(crate) async fn find_subgraph_input(
    mut workflow: WorkflowGraph,
    workflow_run_state: &Arc<dyn WorkflowRunState>,
    asset_storage_registry: &AssetStorageRegistry,
    location: &Location,
) -> Result<WorkflowGraph, miette::Error> {
    let mut parent_location = Location::root();
    for component in location.components() {
        match component {
            crate::location::LocationComponent::Node { node } => {
                let node_states =
                    collect_node_states(workflow_run_state, &workflow, &parent_location).await?;
                let graph_asset_spec =
                    collect_input(&workflow, &node_states, &parent_location, *node, "graph")?;

                let mut inputs = HashMap::new();
                inputs.insert("graph".to_string(), graph_asset_spec);

                workflow = load_subgraph(asset_storage_registry, &inputs).await?;
                parent_location = parent_location.with_node(*node);
            }
            crate::location::LocationComponent::LoopIndex { index } => {
                parent_location = parent_location.with_loop_index(*index);
            }
            crate::location::LocationComponent::MapIndex { index } => {
                parent_location = parent_location.with_map_index(*index as usize);
            }
        }
    }
    Ok(workflow)
}

#[tracing::instrument(
    skip_all,
    fields(location = %parent_location.with_node(node)),
    err,
)]
pub(crate) fn collect_input(
    workflow_graph: &WorkflowGraph,
    node_states: &HashMap<Location, NodeState>,
    parent_location: &Location,
    node: NodeIndex,
    port_name: &str,
) -> miette::Result<AssetSpec> {
    let input_port = workflow_graph.get_input_port_index(node, port_name)?;
    let (linked_node, output_port) = workflow_graph.connected_input(input_port)?;
    let output_name = &workflow_graph.get_port_name(output_port)?;

    let node_state = node_states
        .get(&parent_location.with_node(linked_node))
        .wrap_err_with(|| miette!("Could not find node outputs for node: {linked_node:?}"))?;
    let outputs = node_state
        .outputs
        .as_ref()
        .ok_or_else(|| miette!("Could not find node outputs for node: {linked_node:?}"))?;
    let output_asset_spec = outputs.get(*output_name).ok_or_else(|| {
        let output_keys: Vec<_> = outputs.keys().collect();
        miette!(
            help = format!("Available outputs: {output_keys:?}"),
            "Could not get node output for node: {linked_node:?} and port name: {output_name}",
        )
    })?;

    Ok(output_asset_spec.clone())
}

#[tracing::instrument(
    skip_all,
    fields(location = %parent_location.with_node(node)),
    err,
)]
pub(crate) fn collect_inputs(
    workflow_graph: &WorkflowGraph,
    node_states: &HashMap<Location, NodeState>,
    parent_location: &Location,
    node: NodeIndex,
) -> miette::Result<HashMap<String, AssetSpec>> {
    let mut inputs = HashMap::new();
    for (input_port, output_port) in workflow_graph.input_links(node) {
        let input_name = workflow_graph.get_port_name(input_port.into())?;
        let output_name = workflow_graph.get_port_name(output_port.into())?;
        let linked_node = workflow_graph.port_node(output_port)?;
        let node_state = node_states
            .get(&parent_location.with_node(linked_node))
            .wrap_err_with(|| miette!("Could not find node outputs for node: {linked_node:?}"))?;
        let outputs = node_state
            .outputs
            .as_ref()
            .ok_or_else(|| miette!("Could not find node outputs for node: {linked_node:?}"))?;
        let output_asset_spec = outputs.get(output_name).ok_or_else(|| {
            let output_keys: Vec<_> = outputs.keys().collect();
            miette!(
                help = format!("Available outputs: {output_keys:?}"),
                "Could not get node output for node: {linked_node:?} and port name: {output_name}",
            )
        })?;

        inputs.insert(input_name.clone(), output_asset_spec.clone());
    }
    Ok(inputs)
}
