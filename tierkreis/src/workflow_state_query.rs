//! Utilities for loading and traversing workflow graphs using persisted runtime state.

use std::{
    collections::{HashMap, HashSet},
    ops::BitOrAssign,
    sync::Arc,
};

use bitvec::vec::BitVec;
use miette::{Context, IntoDiagnostic, miette};
use portgraph::NodeIndex;

use crate::{
    asset_storage::{AssetSpec, AssetStorageRegistry, load_asset, load_asset_from_spec},
    graph::{LegacyWorkflowGraph, WorkflowGraph},
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

/// Load a subgraph from the `graph` input, accepting current and legacy formats.
///
/// # Errors
///
/// Returns Err if the graph asset cannot be loaded or decoded.
#[tracing::instrument(skip_all, err)]
pub async fn load_subgraph_from_asset(
    asset_storage_registry: &AssetStorageRegistry,
    asset_spec: AssetSpec,
) -> miette::Result<WorkflowGraph> {
    let subgraph_bytes = load_asset_from_spec(asset_storage_registry, &asset_spec).await?;
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

/// A structure containing the [`Locations`] that have become invalidated by
/// user action, typically during a new workflow run attempt.
#[derive(Default, Debug, Clone)]
pub struct NodeCopyFilter {
    // If a location appears in multiple fields the precedence order is
    // - clear
    // - reset loop
    // - reset map elements
    // - truncate
    //
    // Node states to clear completely + children
    pub(crate) clear_roots: HashSet<Location>,
    // Loop indexes to reset to and clear all subsequent iterations + children
    //
    // These should be the locations of parent Loop nodes.
    pub(crate) reset_loop_index: HashMap<Location, u32>,
    // Map elements to clear from the complete bitset.
    //
    // These should be the locations of parent Map nodes.
    pub(crate) reset_map_elems: HashMap<Location, BitVec<u8>>,
    // Exact node state locations to clear outputs and reset.
    //
    // These should be the locations of parent Eval nodes.
    pub(crate) truncate_exact: HashSet<Location>,
}

impl NodeCopyFilter {
    pub(crate) fn merge(mut self, other: Self) -> Self {
        self.clear_roots.extend(other.clear_roots);
        for (loc, index) in other.reset_loop_index {
            self.reset_loop_index
                .entry(loc)
                .and_modify(|existing| {
                    if index < *existing {
                        *existing = index;
                    }
                })
                .or_insert(index);
        }
        for (loc, bitset) in other.reset_map_elems {
            self.reset_map_elems
                .entry(loc)
                .and_modify(|existing| {
                    let bitset_len = bitset.len();
                    // If we are resetting a lot of map elements it would
                    // be advantegous to resize to the largest map element
                    // first to avoid costly resizing.
                    if bitset_len > existing.len() {
                        existing.resize(bitset_len, false);
                    }
                    existing.bitor_assign(&bitset);
                })
                .or_insert(bitset);
        }
        self.truncate_exact.extend(other.truncate_exact);

        self
    }
}

// Find the locations for which the state would become invalid if a Location's
// state was cleared.
//
// Essentially returns the Locations that would need to be re-computed as they
// are dependent on the result of the specified Location's output.
pub(crate) async fn build_node_copy_filter(
    mut workflow: WorkflowGraph,
    workflow_run_state: &Arc<dyn WorkflowRunState>,
    asset_storage_registry: &AssetStorageRegistry,
    location: Location,
) -> miette::Result<NodeCopyFilter> {
    let mut clear_roots = HashSet::new();
    let mut reset_loop_index = HashMap::new();
    let mut reset_map_elems = HashMap::new();
    let mut truncate_exact = HashSet::new();

    clear_roots.insert(location.clone());

    let mut working_location = Location::root();
    for component in location.components() {
        match component {
            LocationComponent::Node { node } => {
                if !working_location.is_root() {
                    workflow = find_subgraph_input(
                        workflow,
                        workflow_run_state,
                        asset_storage_registry,
                        &working_location,
                    )
                    .await?;
                }

                clear_roots.extend(
                    workflow
                        .dependents(*node)
                        .filter(|dependent| dependent != node)
                        .map(|dependent| working_location.with_node(dependent)),
                );
                working_location = working_location.with_node(*node);

                if working_location != location {
                    truncate_exact.insert(working_location.clone());
                }
            }
            LocationComponent::LoopIndex { index } => {
                reset_loop_index.insert(working_location.clone(), *index);
                working_location = working_location.with_loop_index(*index);
            }
            LocationComponent::MapIndex { index } => {
                let usize_index = *index as usize;
                let mut bitset = BitVec::repeat(false, usize_index + 1);
                bitset.set(usize_index, true);

                reset_map_elems.insert(working_location.clone(), bitset);
                working_location = working_location.with_map_index(usize_index);
            }
        }
    }
    Ok(NodeCopyFilter {
        clear_roots,
        reset_loop_index,
        reset_map_elems,
        truncate_exact,
    })
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

                workflow =
                    load_subgraph_from_asset(asset_storage_registry, graph_asset_spec).await?;
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

#[cfg(test)]
mod tests {
    use bitvec::{bits, order::Lsb0};
    use serde_json::json;
    use uuid::Uuid;

    use crate::{
        asset_storage::test_storage_registry,
        builder::{constant, eval, link, loop_node, map_node, task},
        event::{NodeEvent, NodeStatus, RunningStateUpdate, WorkflowRunEvent},
        state::{InMemoryRuntimeState, RuntimeState},
    };

    use super::*;

    fn example_task_graph() -> miette::Result<WorkflowGraph> {
        let mut graph = WorkflowGraph::new(["out".to_string()]);
        let output_node = graph.output_idx();
        let task_node = task(&mut graph, "foo", "bar", [], ["out"]);
        link(&mut graph, (task_node, "out"), (output_node, "out"))?;

        Ok(graph)
    }

    fn example_triple_task_graph() -> miette::Result<WorkflowGraph> {
        let mut graph = WorkflowGraph::new(["out".to_string()]);
        let output_node = graph.output_idx();
        let const_node_and_name = constant(&mut graph, 1)?;
        let task_node1 = task(&mut graph, "foo", "bar", ["in"], ["out"]);
        let task_node2 = task(&mut graph, "foo", "bar", ["in"], ["out"]);
        let task_node3 = task(&mut graph, "foo", "bar", ["in"], ["out"]);

        link(&mut graph, const_node_and_name, (task_node1, "in"))?;
        link(&mut graph, (task_node1, "out"), (task_node2, "in"))?;
        link(&mut graph, (task_node2, "out"), (task_node3, "in"))?;
        link(&mut graph, (task_node3, "out"), (output_node, "out"))?;

        Ok(graph)
    }

    fn example_eval_graph() -> miette::Result<WorkflowGraph> {
        let mut graph = WorkflowGraph::new(["out".to_string()]);
        let output_node = graph.output_idx();
        let const_node_and_name = constant(&mut graph, example_task_graph()?)?;
        let eval_node = eval(&mut graph, [], ["out"]);

        link(&mut graph, const_node_and_name, (eval_node, "graph"))?;
        link(&mut graph, (eval_node, "out"), (output_node, "out"))?;

        Ok(graph)
    }

    fn example_loop_graph() -> miette::Result<WorkflowGraph> {
        let mut graph = WorkflowGraph::new(["out".to_string()]);
        let output_node = graph.output_idx();
        let const_node_and_name = constant(&mut graph, example_task_graph()?)?;
        let loop_node = loop_node(&mut graph, [], ["out"]);

        link(&mut graph, const_node_and_name, (loop_node, "graph"))?;
        link(&mut graph, (loop_node, "out"), (output_node, "out"))?;

        Ok(graph)
    }

    fn example_map_graph() -> miette::Result<WorkflowGraph> {
        let mut graph = WorkflowGraph::new(["out".to_string()]);
        let output_node = graph.output_idx();
        let const_node_and_name = constant(&mut graph, example_task_graph()?)?;
        let map_node = map_node(&mut graph, [], [], ["out"]);

        link(&mut graph, const_node_and_name, (map_node, "graph"))?;
        link(&mut graph, (map_node, "out"), (output_node, "out"))?;

        Ok(graph)
    }

    #[tokio::test]
    async fn find_invalidated_task() -> miette::Result<()> {
        let (asset_storage_registry, _, _dir) = test_storage_registry([], []).await;
        let state = Arc::new(InMemoryRuntimeState::new());
        let source_state = state
            .new_workflow_run_state(Uuid::now_v7(), HashMap::new())
            .await?;

        let graph = example_task_graph()?;

        let invalid_locations = build_node_copy_filter(
            graph,
            &source_state,
            &asset_storage_registry,
            Location::new("N1")?,
        )
        .await?;

        let exclude = invalid_locations.clear_roots;
        let truncate = invalid_locations.truncate_exact;

        assert_eq!(exclude.len(), 2);
        assert!(exclude.contains(&Location::new("N1")?));
        assert!(exclude.contains(&Location::new("N0")?));
        assert!(truncate.is_empty());

        Ok(())
    }

    #[tokio::test]
    async fn find_invalidated_triple_task_start() -> miette::Result<()> {
        let (asset_storage_registry, _, _dir) = test_storage_registry([], []).await;
        let state = Arc::new(InMemoryRuntimeState::new());
        let source_state = state
            .new_workflow_run_state(Uuid::now_v7(), HashMap::new())
            .await?;

        let graph = example_triple_task_graph()?;

        let invalid_locations = build_node_copy_filter(
            graph,
            &source_state,
            &asset_storage_registry,
            Location::new("N2")?,
        )
        .await?;

        let exclude = invalid_locations.clear_roots;
        let truncate = invalid_locations.truncate_exact;

        dbg!(&exclude);
        assert_eq!(exclude.len(), 4);
        assert!(exclude.contains(&Location::new("N2")?));
        assert!(exclude.contains(&Location::new("N3")?));
        assert!(exclude.contains(&Location::new("N4")?));
        assert!(exclude.contains(&Location::new("N0")?));
        assert!(truncate.is_empty());

        Ok(())
    }

    #[tokio::test]
    async fn find_invalidated_triple_task_end() -> miette::Result<()> {
        let (asset_storage_registry, _, _dir) = test_storage_registry([], []).await;
        let state = Arc::new(InMemoryRuntimeState::new());
        let source_state = state
            .new_workflow_run_state(Uuid::now_v7(), HashMap::new())
            .await?;

        let graph = example_triple_task_graph()?;

        let invalid_locations = build_node_copy_filter(
            graph,
            &source_state,
            &asset_storage_registry,
            Location::new("N4")?,
        )
        .await?;

        let exclude = invalid_locations.clear_roots;
        let truncate = invalid_locations.truncate_exact;

        assert_eq!(exclude.len(), 2);
        assert!(exclude.contains(&Location::new("N4")?));
        assert!(exclude.contains(&Location::new("N0")?));
        assert!(truncate.is_empty());

        Ok(())
    }

    #[tokio::test]
    async fn find_invalidated_task_in_eval() -> miette::Result<()> {
        let (asset_storage_registry, asset_sets, _dir) =
            test_storage_registry([json!({"value": example_task_graph()?})], []).await;
        let state = Arc::new(InMemoryRuntimeState::new());
        let source_state = state
            .new_workflow_run_state(Uuid::now_v7(), HashMap::new())
            .await?;

        let graph = example_eval_graph()?;

        source_state.write(WorkflowRunEvent::Started {}).await?;
        source_state
            .write(WorkflowRunEvent::NodeEvents(vec![NodeEvent {
                loc: Location::new("N1")?,
                status: NodeStatus::Complete {
                    outputs: asset_sets[0].clone(),
                },
                name: None,
            }]))
            .await?;
        source_state
            .write(WorkflowRunEvent::NodeEvents(vec![NodeEvent {
                loc: Location::new("N2")?,
                status: NodeStatus::Running {
                    state_update: Some(RunningStateUpdate::Evalling {
                        inputs: HashMap::new(),
                    }),
                },
                name: None,
            }]))
            .await?;

        let invalid_locations = build_node_copy_filter(
            graph,
            &source_state,
            &asset_storage_registry,
            Location::new("N2.N1")?,
        )
        .await?;

        let exclude = invalid_locations.clear_roots;
        let truncate = invalid_locations.truncate_exact;

        dbg!(&exclude, &truncate);
        assert_eq!(exclude.len(), 3);
        assert!(exclude.contains(&Location::new("N2.N1")?));
        assert!(exclude.contains(&Location::new("N2.N0")?));
        assert!(exclude.contains(&Location::new("N0")?));
        assert_eq!(truncate.len(), 1);
        assert!(truncate.contains(&Location::new("N2")?));

        Ok(())
    }

    #[tokio::test]
    async fn find_invalidated_task_in_loop() -> miette::Result<()> {
        let (asset_storage_registry, asset_sets, _dir) =
            test_storage_registry([json!({"value": example_task_graph()?})], []).await;
        let state = Arc::new(InMemoryRuntimeState::new());
        let source_state = state
            .new_workflow_run_state(Uuid::now_v7(), HashMap::new())
            .await?;

        let graph = example_loop_graph()?;

        source_state.write(WorkflowRunEvent::Started {}).await?;
        source_state
            .write(WorkflowRunEvent::NodeEvents(vec![NodeEvent {
                loc: Location::new("N1")?,
                status: NodeStatus::Complete {
                    outputs: asset_sets[0].clone(),
                },
                name: None,
            }]))
            .await?;
        source_state
            .write(WorkflowRunEvent::NodeEvents(vec![NodeEvent {
                loc: Location::new("N2")?,
                status: NodeStatus::Running {
                    state_update: Some(RunningStateUpdate::Looping {
                        index: 0,
                        inputs: HashMap::new(),
                    }),
                },
                name: None,
            }]))
            .await?;

        let invalid_locations = build_node_copy_filter(
            graph,
            &source_state,
            &asset_storage_registry,
            Location::new("N2.L0.N1")?,
        )
        .await?;

        let exclude = invalid_locations.clear_roots;
        let loop_reset = invalid_locations.reset_loop_index;
        let truncate = invalid_locations.truncate_exact;

        assert_eq!(exclude.len(), 3);
        assert!(exclude.contains(&Location::new("N2.L0.N1")?));
        assert!(exclude.contains(&Location::new("N2.L0.N0")?));
        assert!(exclude.contains(&Location::new("N0")?));
        assert_eq!(loop_reset.len(), 1);
        assert_eq!(loop_reset.get(&Location::new("N2")?), Some(&0));
        assert_eq!(truncate.len(), 1);
        assert!(truncate.contains(&Location::new("N2")?));

        Ok(())
    }

    #[tokio::test]
    async fn find_invalidated_task_in_loop_iteration() -> miette::Result<()> {
        let (asset_storage_registry, asset_sets, _dir) =
            test_storage_registry([json!({"value": example_task_graph()?})], []).await;
        let state = Arc::new(InMemoryRuntimeState::new());
        let source_state = state
            .new_workflow_run_state(Uuid::now_v7(), HashMap::new())
            .await?;

        let graph = example_loop_graph()?;

        source_state.write(WorkflowRunEvent::Started {}).await?;
        source_state
            .write(WorkflowRunEvent::NodeEvents(vec![NodeEvent {
                loc: Location::new("N1")?,
                status: NodeStatus::Complete {
                    outputs: asset_sets[0].clone(),
                },
                name: None,
            }]))
            .await?;
        source_state
            .write(WorkflowRunEvent::NodeEvents(vec![NodeEvent {
                loc: Location::new("N2")?,
                status: NodeStatus::Running {
                    state_update: Some(RunningStateUpdate::Looping {
                        index: 0,
                        inputs: HashMap::new(),
                    }),
                },
                name: None,
            }]))
            .await?;
        source_state
            .write(WorkflowRunEvent::NodeEvents(vec![NodeEvent {
                loc: Location::new("N2")?,
                status: NodeStatus::Running {
                    state_update: Some(RunningStateUpdate::Looping {
                        index: 1,
                        inputs: HashMap::new(),
                    }),
                },
                name: None,
            }]))
            .await?;

        let invalid_locations = build_node_copy_filter(
            graph,
            &source_state,
            &asset_storage_registry,
            Location::new("N2.L1.N1")?,
        )
        .await?;

        let exclude = invalid_locations.clear_roots;
        let loop_reset = invalid_locations.reset_loop_index;
        let truncate = invalid_locations.truncate_exact;

        assert_eq!(exclude.len(), 3);
        assert!(exclude.contains(&Location::new("N2.L1.N1")?));
        assert!(exclude.contains(&Location::new("N2.L1.N0")?));
        assert!(exclude.contains(&Location::new("N0")?));
        assert_eq!(loop_reset.len(), 1);
        assert_eq!(loop_reset.get(&Location::new("N2")?), Some(&1));
        assert_eq!(truncate.len(), 1);
        assert!(truncate.contains(&Location::new("N2")?));

        Ok(())
    }

    #[tokio::test]
    async fn find_invalidated_task_in_loop_mid_iteration() -> miette::Result<()> {
        let (asset_storage_registry, asset_sets, _dir) =
            test_storage_registry([json!({"value": example_task_graph()?})], []).await;
        let state = Arc::new(InMemoryRuntimeState::new());
        let source_state = state
            .new_workflow_run_state(Uuid::now_v7(), HashMap::new())
            .await?;

        let graph = example_loop_graph()?;

        source_state.write(WorkflowRunEvent::Started {}).await?;
        source_state
            .write(WorkflowRunEvent::NodeEvents(vec![NodeEvent {
                loc: Location::new("N1")?,
                status: NodeStatus::Complete {
                    outputs: asset_sets[0].clone(),
                },
                name: None,
            }]))
            .await?;
        source_state
            .write(WorkflowRunEvent::NodeEvents(vec![NodeEvent {
                loc: Location::new("N2")?,
                status: NodeStatus::Running {
                    state_update: Some(RunningStateUpdate::Looping {
                        index: 0,
                        inputs: HashMap::new(),
                    }),
                },
                name: None,
            }]))
            .await?;
        source_state
            .write(WorkflowRunEvent::NodeEvents(vec![NodeEvent {
                loc: Location::new("N2")?,
                status: NodeStatus::Running {
                    state_update: Some(RunningStateUpdate::Looping {
                        index: 1,
                        inputs: HashMap::new(),
                    }),
                },
                name: None,
            }]))
            .await?;
        source_state
            .write(WorkflowRunEvent::NodeEvents(vec![NodeEvent {
                loc: Location::new("N2")?,
                status: NodeStatus::Running {
                    state_update: Some(RunningStateUpdate::Looping {
                        index: 2,
                        inputs: HashMap::new(),
                    }),
                },
                name: None,
            }]))
            .await?;

        let invalid_locations = build_node_copy_filter(
            graph,
            &source_state,
            &asset_storage_registry,
            Location::new("N2.L1.N1")?,
        )
        .await?;

        let exclude = invalid_locations.clear_roots;
        let loop_reset = invalid_locations.reset_loop_index;
        let truncate = invalid_locations.truncate_exact;

        assert_eq!(exclude.len(), 3);
        assert!(exclude.contains(&Location::new("N2.L1.N1")?));
        assert!(exclude.contains(&Location::new("N2.L1.N0")?));
        assert!(exclude.contains(&Location::new("N0")?));
        assert_eq!(loop_reset.len(), 1);
        assert_eq!(loop_reset.get(&Location::new("N2")?), Some(&1));
        assert_eq!(truncate.len(), 1);
        assert!(truncate.contains(&Location::new("N2")?));

        Ok(())
    }

    #[tokio::test]
    async fn find_invalidated_task_in_map() -> miette::Result<()> {
        let (asset_storage_registry, asset_sets, _dir) =
            test_storage_registry([json!({"value": example_task_graph()?})], []).await;
        let state = Arc::new(InMemoryRuntimeState::new());
        let source_state = state
            .new_workflow_run_state(Uuid::now_v7(), HashMap::new())
            .await?;

        let graph = example_map_graph()?;

        source_state.write(WorkflowRunEvent::Started {}).await?;
        source_state
            .write(WorkflowRunEvent::NodeEvents(vec![NodeEvent {
                loc: Location::new("N1")?,
                status: NodeStatus::Complete {
                    outputs: asset_sets[0].clone(),
                },
                name: None,
            }]))
            .await?;
        source_state
            .write(WorkflowRunEvent::NodeEvents(vec![NodeEvent {
                loc: Location::new("N2")?,
                status: NodeStatus::Running {
                    state_update: Some(RunningStateUpdate::MapStarted {
                        input_sets: vec![HashMap::new(), HashMap::new(), HashMap::new()],
                    }),
                },
                name: None,
            }]))
            .await?;

        let invalid_locations1 = build_node_copy_filter(
            graph.clone(),
            &source_state,
            &asset_storage_registry,
            Location::new("N2.M0.N1")?,
        )
        .await?;

        let exclude = &invalid_locations1.clear_roots;
        let map_reset = &invalid_locations1.reset_map_elems;
        let truncate = &invalid_locations1.truncate_exact;

        assert_eq!(exclude.len(), 3);
        assert!(exclude.contains(&Location::new("N2.M0.N1")?));
        assert!(exclude.contains(&Location::new("N2.M0.N0")?));
        assert!(exclude.contains(&Location::new("N0")?));
        assert_eq!(map_reset.len(), 1);
        assert_eq!(
            map_reset.get(&Location::new("N2")?),
            Some(&BitVec::from_bitslice(bits![u8, Lsb0; 1]))
        );
        assert_eq!(truncate.len(), 1);
        assert!(truncate.contains(&Location::new("N2")?));

        let invalid_locations2 = build_node_copy_filter(
            graph,
            &source_state,
            &asset_storage_registry,
            Location::new("N2.M2.N1")?,
        )
        .await?;

        let exclude = &invalid_locations2.clear_roots;
        let map_reset = &invalid_locations2.reset_map_elems;
        let truncate = &invalid_locations2.truncate_exact;

        assert_eq!(exclude.len(), 3);
        assert!(exclude.contains(&Location::new("N2.M2.N1")?));
        assert!(exclude.contains(&Location::new("N2.M2.N0")?));
        assert!(exclude.contains(&Location::new("N0")?));
        assert_eq!(map_reset.len(), 1);
        assert_eq!(
            map_reset.get(&Location::new("N2")?),
            Some(&BitVec::from_bitslice(bits![u8, Lsb0; 0, 0, 1]))
        );
        assert_eq!(truncate.len(), 1);
        assert!(truncate.contains(&Location::new("N2")?));

        let merged_invalid_locations = invalid_locations1.merge(invalid_locations2);

        let exclude = merged_invalid_locations.clear_roots;
        let map_reset = merged_invalid_locations.reset_map_elems;
        let truncate = merged_invalid_locations.truncate_exact;

        assert_eq!(exclude.len(), 5);
        assert!(exclude.contains(&Location::new("N2.M0.N1")?));
        assert!(exclude.contains(&Location::new("N2.M0.N0")?));
        assert!(exclude.contains(&Location::new("N2.M2.N1")?));
        assert!(exclude.contains(&Location::new("N2.M2.N0")?));
        assert!(exclude.contains(&Location::new("N0")?));
        assert_eq!(map_reset.len(), 1);
        assert_eq!(
            map_reset.get(&Location::new("N2")?),
            Some(&BitVec::from_bitslice(bits![u8, Lsb0; 1, 0, 1]))
        );
        assert_eq!(truncate.len(), 1);
        assert!(truncate.contains(&Location::new("N2")?));

        Ok(())
    }
}
