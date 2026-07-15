/*!
This module defines the [Orchestrator] struct that combines multiple [Executor][crate::executor::Executor]
and [`AssetStorage`][crate::asset_storage::AssetStorage] implementations to drive Workflow execution and return a stream
of [Event]s with updates about each node in the Workflow.
*/
use std::{
    collections::{HashMap, HashSet},
    iter,
    sync::{Arc, Mutex},
};

use bitvec::vec::BitVec;
use futures::{
    FutureExt, Stream, StreamExt, TryFutureExt,
    channel::mpsc,
    future,
    stream::{self, BoxStream, LocalBoxStream, select_all},
};
use miette::{Context, IntoDiagnostic, miette};
use portgraph::{NodeIndex, PortIndex};
use serde_json::Value;
use tracing::{debug, instrument};
use uuid::Uuid;

use crate::{
    asset_storage::{
        AssetStorageRegistry, fold_assets, interface::AssetSpec, load_asset, save_asset,
        unfold_asset,
    },
    event::{
        EventReceiver, EventSender, NodeEvent, RuntimeEvent, WorkflowRunEvent, send_complete,
        send_map_elem_complete, send_running_loop, send_running_map, send_running_switching,
    },
    executor::{
        ExecutorRegistry, HPCExecutor,
        hpc::{HPCEnvironmentSpec, HPCResourceSpec},
        interface::TaskPlan,
    },
    graph::{LegacyWorkflowGraph, NodeDefinition, WorkflowGraph},
    location::Location,
    state::{WorkflowRunState, interface::NodeState},
};

/// `Action` describes an operation the Orchestrator should perform.
#[derive(Debug, Clone, PartialEq)]
pub struct Action {
    /// The node location in the graph where orchestration should occur.
    pub loc: Location,
    /// The kind of action to perform.
    pub kind: ActionKind,
}

/// [`ActionKind`] is a placeholder enum for operation the [`Orchestrator`] can perform
/// when interpreting a [`WorkflowGraph`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ActionKind {
    /// An operation performed by a Worker.
    PerformTask {
        /// The name of the Worker to call.
        worker_name: String,
        /// The name of the Task to call.
        task_name: String,
        /// Arbitrary resource requirements used for executor selection.
        resources: HashMap<String, Value>,
        /// Arbitrary environment requirements used for executor selection.
        environment: HashMap<String, Value>,
        /// The input assets for the Task.
        inputs: HashMap<String, AssetSpec>,
        /// The names of the outputs of the Task.
        outputs: HashSet<String>,
    },
    /// Mark the node as switching with a particular value.
    SetSwitching {
        /// The value to mark the node with.
        cond: bool,
    },
    /// Mark the node as running with a particular loop index.
    SetRunningLoop {
        /// The loop index to store in the node state.
        index: u32,
    },
    /// Mark the node as running with a particular map size.
    SetRunningMap {
        /// The size of the map to mark.
        size: usize,
    },
    /// Mark the node as partially complete for a particular element.
    SetMapElemComplete {
        /// The size of the map.
        size: usize,
        /// The map element to mark as complete.
        index: usize,
    },
    /// Mark the node as complete with outputs.
    SetComplete {
        /// The output values for the node.
        outputs: HashMap<String, AssetSpec>,
    },
    /// Mark the overall workflow as complete.
    WorkflowFinished {},
}

#[derive(Debug, Clone, Default)]
struct ActionPlan {
    tasks: Vec<TaskPlan>,
    switching: Vec<(Location, bool)>,
    looping: Vec<(Location, u32)>,
    mapping: Vec<(Location, usize)>,
    map_elem_complete: HashMap<Location, BitVec<u8>>,
    node_complete: Vec<(Location, HashMap<String, AssetSpec>)>,
    workflow_complete: bool,
}

/// The context state for the orchestration
#[derive(Debug)]
pub struct OrchestrationContext {
    parent_loc: Location,
    graph_inputs: HashMap<String, AssetSpec>,
    workflow_run_state: Arc<dyn WorkflowRunState>,
}

impl Clone for OrchestrationContext {
    fn clone(&self) -> Self {
        Self {
            parent_loc: self.parent_loc.clone(),
            graph_inputs: self.graph_inputs.clone(),
            workflow_run_state: Arc::clone(&self.workflow_run_state),
        }
    }
}

impl OrchestrationContext {
    /// Construct a new [`OrchestrationContext`] at the root Location.
    pub fn new(
        workflow_run_state: &Arc<dyn WorkflowRunState>,
        inputs: HashMap<String, AssetSpec>,
    ) -> Self {
        Self {
            parent_loc: Location::root(),
            graph_inputs: inputs,
            workflow_run_state: Arc::clone(workflow_run_state),
        }
    }
}

type NodeStates = HashMap<Location, NodeState>;

/// [Orchestrator] manages the Workflow execution by dispatching [Node]s to the correct
/// [Executor][crate::executor::Executor] as well as managing a shared [`AssetStorageRegistry`]
/// for the Workflow.
pub struct Orchestrator {
    event_sender: EventSender,
    event_receiver: Mutex<Option<EventReceiver>>,

    default_executor_name: String,
    executor_registry: ExecutorRegistry,

    default_storage_name: String,
    asset_storage_registry: AssetStorageRegistry,
}

impl Orchestrator {
    /// Try to create a new [Orchestrator] with an [`AssetStorageRegistry`] and an
    /// [`ExecutorRegistry`], as well as default options for the [`AssetStorage`][crate::asset_storage::AssetStorage]
    /// and [Executor][crate::executor::Executor] to use for each registry unless specified otherwise
    /// in the Workflow definition.
    ///
    /// # Errors
    ///
    /// This function will return Err if the specified `default_storage_name` does not exist
    /// inside the [`AssetStorageRegistry`] or if the specified `default_executor_name` does
    /// not exist inside the [`ExecutorRegistry`].
    pub async fn try_new(
        asset_storage_registry: &AssetStorageRegistry,
        executor_registry: &ExecutorRegistry,
        default_storage_name: &str,
        default_executor_name: &str,
    ) -> miette::Result<Self> {
        let (sender, receiver) = mpsc::channel(128);

        let asset_storage_registry_lock = asset_storage_registry.read().await;
        if !asset_storage_registry_lock.contains_key(default_storage_name) {
            return Err(miette!("default_storage_name not in registry"));
        }

        if !executor_registry.contains_key(default_executor_name) {
            return Err(miette!("default_executor_name not in registry"));
        }

        Ok(Self {
            event_sender: sender,
            event_receiver: Mutex::new(Some(receiver)),

            default_executor_name: default_executor_name.to_string(),
            executor_registry: Arc::clone(executor_registry),

            default_storage_name: default_storage_name.to_string(),
            asset_storage_registry: Arc::clone(asset_storage_registry),
        })
    }

    /// Create a stream of [`Action`]s based on [`OrchstrationContext`] and a [`WorkflowGraph`].
    ///
    /// This function effectively flattens higher order graph execution into a stream of [`Action`]s
    /// that can then be processed by the [`perform_actions`] method.
    ///
    /// # Errors
    ///
    /// Will return Err if the function fails to retrieve the state of nodes from the workflow context
    /// or if it fails to record that nodes are being scheduled.
    #[instrument(skip_all, err)]
    pub async fn build_actions<'a>(
        &'a self,
        context: OrchestrationContext,
        workflow_graph: Arc<WorkflowGraph>,
    ) -> miette::Result<LocalBoxStream<'a, miette::Result<Action>>> {
        let node_states = Self::collect_node_states(&context, &workflow_graph).await?;

        if let Some(output_state) =
            node_states.get(&context.parent_loc.with_node(workflow_graph.output_idx()))
            && output_state.scheduled_time.is_some()
        {
            // Output is already scheduled, no actions to perform.
            return Ok(stream::empty().boxed());
        }

        let ready_nodes: Vec<_> =
            Self::find_ready_nodes(&context.parent_loc, &workflow_graph, &node_states).collect();
        // Mark all ready nodes as scheduled.
        Self::mark_nodes_scheduled(&context, ready_nodes.iter()).await?;

        let node_states = Arc::new(node_states);
        Ok(stream::iter(ready_nodes)
            .flat_map_unordered(None, move |n| -> LocalBoxStream<miette::Result<Action>> {
                let node_states = Arc::clone(&node_states);
                let Some(definition) = workflow_graph.node_definition(n) else {
                    return stream_error(miette!("Could not find node definition"));
                };

                let parent_location = context.parent_loc.clone();
                match definition {
                    NodeDefinition::Input { name } => stream_action_result(
                        Self::build_input_action(&context.graph_inputs, parent_location, n, name),
                    ),
                    NodeDefinition::Const { value } => self
                        .build_const_action(parent_location, n, value.clone())
                        .into_stream()
                        .boxed(),
                    NodeDefinition::Output {} => self
                        .build_output_actions(
                            workflow_graph.clone(),
                            node_states,
                            parent_location,
                            n,
                        )
                        .try_flatten_stream()
                        .boxed(),
                    NodeDefinition::Task {
                        worker_name,
                        task_name,
                    } => stream_action_result(build_task_action(
                        workflow_graph.clone(),
                        &node_states,
                        &parent_location,
                        n,
                        worker_name,
                        task_name,
                    )),
                    NodeDefinition::Eval {} => self
                        .build_eval_actions(
                            workflow_graph.clone(),
                            context.workflow_run_state.clone(),
                            node_states,
                            parent_location,
                            n,
                        )
                        .try_flatten_stream()
                        .boxed_local(),
                    NodeDefinition::Loop {} => self
                        .build_loop_actions(
                            workflow_graph.clone(),
                            context.workflow_run_state.clone(),
                            node_states,
                            parent_location,
                            n,
                        )
                        .try_flatten_stream()
                        .boxed_local(),
                    NodeDefinition::Map { mapped_ports } => self
                        .build_map_actions(
                            workflow_graph.clone(),
                            context.workflow_run_state.clone(),
                            node_states,
                            mapped_ports.clone(),
                            parent_location,
                            n,
                        )
                        .try_flatten_stream()
                        .boxed_local(),
                    // Eager and Lazy If else are controlled by the ready node checks
                    NodeDefinition::IfElse {} => self
                        .build_if_else_action(
                            workflow_graph.clone(),
                            node_states,
                            parent_location,
                            n,
                        )
                        .into_stream()
                        .boxed(),
                    NodeDefinition::EagerIfElse {} => self
                        .build_eager_if_else_action(
                            workflow_graph.clone(),
                            node_states,
                            parent_location,
                            n,
                        )
                        .into_stream()
                        .boxed(),
                }
            })
            .boxed_local())
    }

    async fn collect_node_states(
        context: &OrchestrationContext,
        workflow_graph: &Arc<WorkflowGraph>,
    ) -> miette::Result<NodeStates> {
        context
            .workflow_run_state
            .read_many(
                &mut workflow_graph
                    .node_ids()
                    .map(|n| context.parent_loc.with_node(n)),
            )
            .await
    }

    /// Find nodes which are ready for execution, mark them as scheduled then return them.
    fn find_ready_nodes<'a>(
        parent_location: &'a Location,
        workflow_graph: &'a WorkflowGraph,
        node_states: &'a NodeStates,
    ) -> impl Iterator<Item = NodeIndex> {
        // Find nodes that are ready for scheduling.
        workflow_graph
            // Traverse the graph to find nodes that are yet to run, starting
            // from the output node.
            //
            // TODO: We shouldn't really need to sort every time
            .toposort_filtered_from_output_node(
                // Returns true if a node should be traversed.
                |n| {
                    let definition = workflow_graph
                        .node_definition(n)
                        .expect("Node definition not found");

                    if let Some(state) = node_states.get(&parent_location.with_node(n)) {
                        // TODO: This sometimes means even const/input nodes
                        // are run multiple times if their state is yet
                        // to be updated from the last orchestration round.
                        state.outputs.is_none()
                            && !(matches!(
                                definition,
                                NodeDefinition::Task { .. }
                                    | NodeDefinition::Input { .. }
                                    | NodeDefinition::Const { .. }
                                    | NodeDefinition::Output {}
                            ) && state.scheduled_time.is_some())
                    } else {
                        true
                    }
                },
                // Returns true if a port should be traversed.
                |n, p| {
                    let definition = workflow_graph
                        .node_definition(n)
                        .expect("Node definition not found");
                    if matches!(definition, NodeDefinition::IfElse {}) {
                        let cond = node_states
                            .get(&parent_location.with_node(n))
                            .and_then(|state| state.cond);
                        should_traverse_if_else_port(workflow_graph, cond, p)
                    } else {
                        true
                    }
                },
            )
            // Of the nodes that have not yet run, find the nodes that can run.
            // (Because their inputs are ready or otherwise.)
            .filter(|n| {
                let definition = workflow_graph
                    .node_definition(*n)
                    .expect("Node definition not found");
                if matches!(definition, NodeDefinition::IfElse {}) {
                    Self::if_else_ready(workflow_graph, node_states, parent_location, *n)
                } else {
                    workflow_graph.all_inputs(*n, |incoming| {
                        node_states
                            .get(&parent_location.with_node(incoming))
                            .is_some_and(|state| state.outputs.is_some())
                    })
                }
            })
    }

    // Returns true if an `IfElse` node is runnable.
    fn if_else_ready(
        workflow_graph: &WorkflowGraph,
        node_states: &NodeStates,
        parent_location: &Location,
        n: NodeIndex,
    ) -> bool {
        let cond = node_states
            .get(&parent_location.with_node(n))
            .and_then(|state| state.cond);
        match cond {
            None => Self::port_has_input(workflow_graph, node_states, parent_location, n, "pred")
                .expect("No `pred` port on `IfElse` node"),
            Some(true) => {
                Self::port_has_input(workflow_graph, node_states, parent_location, n, "if_true")
                    .expect("No `if_true` port on `IfElse` node")
            }
            Some(false) => {
                Self::port_has_input(workflow_graph, node_states, parent_location, n, "if_false")
                    .expect("No `if_false` port on `IfElse` node")
            }
        }
    }

    // Returns true if a port on a node's input is available.
    fn port_has_input(
        workflow_graph: &WorkflowGraph,
        node_states: &NodeStates,
        parent_location: &Location,
        n: NodeIndex,
        port_name: &str,
    ) -> miette::Result<bool> {
        let (connected_node, _) = workflow_graph.connected_input_by_port_name(n, port_name)?;

        Ok(node_states
            .get(&parent_location.with_node(connected_node))
            .is_some_and(|state| state.outputs.is_some()))
    }

    async fn mark_nodes_scheduled(
        context: &OrchestrationContext,
        nodes: impl Iterator<Item = &NodeIndex>,
    ) -> miette::Result<()> {
        context
            .workflow_run_state
            .write(WorkflowRunEvent::NodeEvent(NodeEvent {
                locs: nodes.map(|n| context.parent_loc.with_node(*n)).collect(),
                status: crate::event::NodeStatus::Scheduled {},
            }))
            .await?;

        Ok(())
    }

    #[instrument(
        skip_all,
        fields(location = %parent_location.with_node(n)),
        err,
    )]
    async fn build_if_else_action(
        &self,
        workflow_graph: Arc<WorkflowGraph>,
        node_states: Arc<NodeStates>,
        parent_location: Location,
        n: NodeIndex,
    ) -> miette::Result<Action> {
        let cond = node_states
            .get(&parent_location.with_node(n))
            .and_then(|state| state.cond);
        match cond {
            None => {
                let (pred_node, connected_port) =
                    workflow_graph.connected_input_by_port_name(n, "pred")?;
                let pred_state = node_states
                    .get(&parent_location.with_node(pred_node))
                    .ok_or_else(|| miette!("Cannot find state for `pred` node"))?;
                let connected_port_name = workflow_graph.get_port_name(connected_port)?;

                let pred_bytes = load_asset(
                    &self.asset_storage_registry,
                    pred_state
                        .outputs
                        .as_ref()
                        .ok_or_else(|| miette!("`pred` node has no outputs"))?,
                    connected_port_name,
                )
                .await?;

                Ok(Action {
                    loc: parent_location.with_node(n),
                    kind: ActionKind::SetSwitching {
                        cond: pred_bytes == b"true",
                    },
                })
            }
            Some(true) => Self::build_branch_action(
                &workflow_graph,
                &node_states,
                &parent_location,
                n,
                "if_true",
            ),
            Some(false) => Self::build_branch_action(
                &workflow_graph,
                &node_states,
                &parent_location,
                n,
                "if_false",
            ),
        }
    }

    fn build_branch_action(
        workflow_graph: &Arc<WorkflowGraph>,
        node_states: &NodeStates,
        parent_location: &Location,
        n: NodeIndex,
        port_name: &str,
    ) -> miette::Result<Action> {
        let (connected_node, connected_port) =
            workflow_graph.connected_input_by_port_name(n, port_name)?;
        let node_state = node_states
            .get(&parent_location.with_node(connected_node))
            .ok_or_else(|| miette!("Cannot find state for `{port_name}` node"))?;
        let connected_port_name = workflow_graph.get_port_name(connected_port)?;

        let mut outputs = HashMap::new();
        let value = node_state
            .outputs
            .as_ref()
            .ok_or_else(|| miette!("No outputs found on node connected to `{port_name}` port."))?
            .get(connected_port_name)
            .ok_or_else(|| miette!("No outputs found for `{connected_port_name}` port."))?;
        outputs.insert("value".to_string(), value.clone());

        Ok(Action {
            loc: parent_location.with_node(n),
            kind: ActionKind::SetComplete { outputs },
        })
    }

    #[instrument(
        skip_all,
        fields(location = %parent_location.with_node(n)),
        err,
    )]
    async fn build_eager_if_else_action(
        &self,
        workflow_graph: Arc<WorkflowGraph>,
        node_states: Arc<NodeStates>,
        parent_location: Location,
        n: NodeIndex,
    ) -> miette::Result<Action> {
        let mut inputs = collect_inputs(&workflow_graph, &node_states, &parent_location, n)?;

        let pred_bytes = load_asset(&self.asset_storage_registry, &inputs, "pred").await?;
        let mut outputs = HashMap::new();

        if pred_bytes == b"true" {
            let value = inputs.remove("if_true").unwrap();
            outputs.insert("value".to_string(), value);
        } else {
            let value = inputs.remove("if_false").unwrap();
            outputs.insert("value".to_string(), value);
        }

        Ok(Action {
            loc: parent_location.with_node(n),
            kind: ActionKind::SetComplete { outputs },
        })
    }

    #[instrument(
        skip_all,
        fields(location = %parent_location.with_node(n), name = name),
        err,
    )]
    fn build_input_action(
        graph_inputs: &HashMap<String, AssetSpec>,
        parent_location: Location,
        n: NodeIndex,
        name: &str,
    ) -> miette::Result<Action> {
        let value = graph_inputs
            .get(name)
            .ok_or_else(|| miette!("Input not found for port: {name}"))?;
        let mut outputs = HashMap::new();
        outputs.insert(name.to_string(), value.clone());

        Ok(Action {
            loc: parent_location.with_node(n),
            kind: ActionKind::SetComplete { outputs },
        })
    }

    #[instrument(
        skip_all,
        fields(location = %parent_location.with_node(n)),
        err,
    )]
    async fn build_const_action(
        &self,
        parent_location: Location,
        n: NodeIndex,
        value: serde_json::Value,
    ) -> miette::Result<Action> {
        let value_bytes = serde_json::to_vec(&value).into_diagnostic()?;
        let asset_key = save_asset(
            &self.asset_storage_registry,
            &self.default_storage_name,
            value_bytes,
        )
        .await?;

        let mut outputs = HashMap::new();
        outputs.insert("value".to_string(), asset_key);
        Ok(Action {
            loc: parent_location.with_node(n),
            kind: ActionKind::SetComplete { outputs },
        })
    }

    #[instrument(
        skip_all,
        fields(location = %parent_location.with_node(n)),
        err,
    )]
    async fn build_output_actions(
        &self,
        workflow_graph: Arc<WorkflowGraph>,
        node_states: Arc<NodeStates>,
        parent_location: Location,
        n: NodeIndex,
    ) -> miette::Result<BoxStream<'_, miette::Result<Action>>> {
        let inputs = collect_inputs(&workflow_graph, &node_states, &parent_location, n)?;
        let loc = parent_location.with_node(n);

        let is_root_workflow = loc == Location::from_node_index_iter([workflow_graph.output_idx()]);
        let mut actions = vec![Ok(Action {
            loc: loc.clone(),
            kind: ActionKind::SetComplete { outputs: inputs },
        })];

        if is_root_workflow {
            actions.push(Ok(Action {
                loc,
                kind: ActionKind::WorkflowFinished {},
            }));
        }

        Ok(stream::iter(actions).boxed())
    }

    #[instrument(
        skip_all,
        fields(location = %parent_location.with_node(n)),
        err,
    )]
    async fn build_eval_actions<'a>(
        &'a self,
        workflow_graph: Arc<WorkflowGraph>,
        workflow_run_state: Arc<dyn WorkflowRunState>,
        node_states: Arc<NodeStates>,
        parent_location: Location,
        n: NodeIndex,
    ) -> miette::Result<LocalBoxStream<'a, miette::Result<Action>>> {
        // TODO: we don't need to collect inputs other than the graph
        // itself if the graph has already started.
        let inputs = collect_inputs(&workflow_graph, &node_states, &parent_location, n)?;
        let loc = parent_location.with_node(n);

        let subgraph = if inputs.contains_key("graph") {
            self.load_subgraph(&inputs).await?
        } else {
            workflow_graph
        };

        let subgraph_output_state = workflow_run_state
            .read(&loc.with_node(subgraph.output_idx()))
            .await?;

        if let Some(subgraph_outputs) = subgraph_output_state.outputs {
            return Ok(stream::once(future::ok(Action {
                loc,
                kind: ActionKind::SetComplete {
                    outputs: subgraph_outputs,
                },
            }))
            .boxed());
        }

        let stream = self
            .build_actions(
                OrchestrationContext {
                    parent_loc: loc,
                    graph_inputs: inputs,
                    workflow_run_state: workflow_run_state.clone(),
                },
                subgraph,
            )
            .await?;

        Ok(stream.boxed_local())
    }

    #[instrument(
        skip_all,
        fields(location = %parent_location.with_node(n)),
        err,
    )]
    async fn build_loop_actions<'a>(
        &'a self,
        workflow_graph: Arc<WorkflowGraph>,
        workflow_run_state: Arc<dyn WorkflowRunState>,
        node_states: Arc<NodeStates>,
        parent_location: Location,
        n: NodeIndex,
    ) -> miette::Result<LocalBoxStream<'a, miette::Result<Action>>> {
        let loc = parent_location.with_node(n);
        let loop_index = node_states.get(&loc).and_then(|state| state.loop_index);
        match loop_index {
            None => {
                return Ok(stream::once(future::ok(Action {
                    loc,
                    kind: ActionKind::SetRunningLoop { index: 0 },
                }))
                .boxed());
            }
            Some(index) => {
                // TODO: We probably don't need the inputs if we have already
                // visited this loop iteration.
                let mut inputs =
                    collect_inputs(&workflow_graph, &node_states, &parent_location, n)?;
                let subgraph = self.load_subgraph(&inputs).await?;

                let loop_loc = loc.with_loop_index(index);
                let loop_subgraph_output_loc = loop_loc.with_node(subgraph.output_idx());
                let loop_iteration_output_state =
                    workflow_run_state.read(&loop_subgraph_output_loc).await?;

                match loop_iteration_output_state.outputs {
                    None => {
                        if index > 0 {
                            let prev_loop_loc = loc.with_loop_index(index - 1);
                            let prev_loop_subgraph_output_loc =
                                prev_loop_loc.with_node(subgraph.output_idx());
                            let prev_loop_iteration_output_state = workflow_run_state
                                .read(&prev_loop_subgraph_output_loc)
                                .await?;

                            inputs.extend(prev_loop_iteration_output_state.outputs.ok_or_else(
                                || miette!("No outputs from previous loop iteration"),
                            )?);
                        }

                        return self
                            .build_actions(
                                OrchestrationContext {
                                    parent_loc: loop_loc,
                                    graph_inputs: inputs,
                                    workflow_run_state: Arc::clone(&workflow_run_state),
                                },
                                subgraph,
                            )
                            .await;
                    }
                    Some(outputs) => {
                        let should_continue_bytes =
                            load_asset(&self.asset_storage_registry, &outputs, "should_continue")
                                .await?;

                        if should_continue_bytes == b"true" {
                            Ok(stream::once(future::ok(Action {
                                loc,
                                kind: ActionKind::SetRunningLoop { index: index + 1 },
                            }))
                            .boxed())
                        } else {
                            Ok(stream::once(future::ok(Action {
                                loc,
                                kind: ActionKind::SetComplete { outputs },
                            }))
                            .boxed())
                        }
                    }
                }
            }
        }
    }

    #[instrument(
        skip_all,
        fields(location = %parent_location.with_node(n)),
        err,
    )]
    async fn build_map_actions<'a>(
        &'a self,
        workflow_graph: Arc<WorkflowGraph>,
        workflow_run_state: Arc<dyn WorkflowRunState>,
        node_states: Arc<NodeStates>,
        mapped_ports: HashSet<String>,
        parent_location: Location,
        n: NodeIndex,
    ) -> miette::Result<LocalBoxStream<'a, miette::Result<Action>>> {
        let completed = node_states
            .get(&parent_location.with_node(n))
            .and_then(|state| state.map_completed.as_deref());
        let inputs = collect_inputs(&workflow_graph, &node_states, &parent_location, n)?;
        let subgraph = self.load_subgraph(&inputs).await?;

        Ok(match completed {
            None => {
                self.build_initial_map_actions(
                    workflow_run_state,
                    mapped_ports,
                    parent_location.with_node(n),
                    inputs,
                    subgraph,
                )
                .await?
            }
            Some(completed) if completed.all() => self
                .build_completed_map_action(
                    workflow_graph,
                    workflow_run_state,
                    parent_location,
                    n,
                    completed.len(),
                    subgraph.output_idx(),
                )
                .into_stream()
                .boxed_local(),
            Some(completed) => {
                self.build_subsequent_map_actions(
                    workflow_run_state,
                    parent_location.with_node(n),
                    completed.to_bitvec(),
                    inputs,
                    subgraph,
                )
                .await?
            }
        })
    }

    #[instrument(
        skip_all,
        fields(location = %location, mapped_ports = ?mapped_ports),
        err,
    )]
    async fn build_initial_map_actions<'a>(
        &'a self,
        workflow_run_state: Arc<dyn WorkflowRunState>,
        mapped_ports: HashSet<String>,
        location: Location,
        inputs: HashMap<String, AssetSpec>,
        subgraph: Arc<WorkflowGraph>,
    ) -> miette::Result<LocalBoxStream<'a, miette::Result<Action>>> {
        let mut input_sets = Vec::new();
        for mapped_port in mapped_ports {
            let unfolded_assets =
                unfold_asset(&self.asset_storage_registry, &inputs, &mapped_port).await?;

            if input_sets.is_empty() {
                input_sets.extend(iter::repeat_n(inputs.clone(), unfolded_assets.len()));
            }

            for (index, asset) in unfolded_assets.into_iter().enumerate() {
                let input_set = input_sets.get_mut(index).unwrap();
                input_set.insert(mapped_port.clone(), asset);
            }
        }
        let map_size = input_sets.len();

        let loc_copy = location.clone();

        Ok(stream::iter(input_sets.into_iter().enumerate())
            .flat_map_unordered(None, move |(index, inputs)| {
                let map_loc = loc_copy.with_map_index(index);
                self.build_actions(
                    OrchestrationContext {
                        parent_loc: map_loc,
                        graph_inputs: inputs,
                        workflow_run_state: Arc::clone(&workflow_run_state),
                    },
                    subgraph.clone(),
                )
                .try_flatten_stream()
                .boxed_local()
            })
            .chain(stream::once(future::ok(Action {
                loc: location,
                kind: ActionKind::SetRunningMap { size: map_size },
            })))
            .boxed_local())
    }

    #[instrument(
        skip_all,
        fields(location = %location),
        err,
    )]
    async fn build_subsequent_map_actions<'a>(
        &'a self,
        workflow_run_state: Arc<dyn WorkflowRunState>,
        location: Location,
        completed: BitVec<u8>,
        inputs: HashMap<String, AssetSpec>,
        subgraph: Arc<WorkflowGraph>,
    ) -> miette::Result<LocalBoxStream<'a, miette::Result<Action>>> {
        let output_idx = subgraph.output_idx();
        let map_size = completed.len();

        Ok(stream::iter(completed.into_iter().enumerate())
            .filter(|(_index, completed)| future::ready(!completed))
            .flat_map_unordered(None, move |(index, _completed)| {
                let loc_copy = location.clone();
                let map_loc = location.with_map_index(index);
                let map_loc_copy = map_loc.clone();
                self.build_actions(
                    OrchestrationContext {
                        parent_loc: map_loc,
                        graph_inputs: inputs.clone(),
                        workflow_run_state: Arc::clone(&workflow_run_state),
                    },
                    subgraph.clone(),
                )
                .try_flatten_stream()
                .chain({
                    let workflow_run_state_copy = workflow_run_state.clone();
                    async move {
                        let subgraph_output_state = workflow_run_state_copy
                            .read(&map_loc_copy.with_node(output_idx))
                            .await?;

                        match subgraph_output_state.outputs {
                            None => Ok(stream::empty().boxed()),
                            Some(_) => Ok(stream::once(future::ok(Action {
                                loc: loc_copy,
                                kind: ActionKind::SetMapElemComplete {
                                    index,
                                    size: map_size,
                                },
                            }))
                            .boxed()),
                        }
                    }
                    .try_flatten_stream()
                })
                .boxed_local()
            })
            .boxed_local())
    }

    #[instrument(
        skip_all,
        fields(location = %parent_location.with_node(n)),
        err,
    )]
    async fn build_completed_map_action(
        &self,
        workflow_graph: Arc<WorkflowGraph>,
        workflow_run_state: Arc<dyn WorkflowRunState>,
        parent_location: Location,
        n: NodeIndex,
        map_size: usize,
        output_idx: NodeIndex,
    ) -> miette::Result<Action> {
        let loc = parent_location.with_node(n);
        let mut asset_bundles: HashMap<String, Vec<_>> = workflow_graph
            .output_names(n)?
            .map(|name| (name.clone(), Vec::new()))
            .collect();
        for index in 0..map_size {
            let map_loc = loc.with_map_index(index);
            let subgraph_output_state = workflow_run_state
                .read(&map_loc.with_node(output_idx))
                .await?;

            let outputs = subgraph_output_state
                .outputs
                .ok_or_else(|| miette!("No outputs!"))?;

            for (k, v) in outputs {
                let entry = asset_bundles.entry(k).or_default();
                entry.push(v);
            }
        }

        let mut outputs = HashMap::new();
        for (name, asset_specs) in asset_bundles {
            let folded_asset_spec = fold_assets(
                &self.asset_storage_registry,
                &self.default_storage_name,
                asset_specs,
            )
            .await?;
            outputs.insert(name, folded_asset_spec);
        }

        Ok(Action {
            loc,
            kind: ActionKind::SetComplete { outputs },
        })
    }

    #[instrument(skip_all, err)]
    async fn load_subgraph(
        &self,
        inputs: &HashMap<String, AssetSpec>,
    ) -> miette::Result<Arc<WorkflowGraph>> {
        let subgraph_bytes = load_asset(&self.asset_storage_registry, inputs, "graph").await?;
        let subgraph_res: Result<WorkflowGraph, serde_json::Error> =
            serde_json::from_slice(&subgraph_bytes);

        let subgraph = match subgraph_res {
            Ok(subgraph) => subgraph,
            Err(_err) => {
                // TODO: rich error message here
                let legacy: LegacyWorkflowGraph =
                    serde_json::from_slice(&subgraph_bytes).into_diagnostic()?;
                legacy.to_workflow_graph()?
            }
        };

        Ok(Arc::new(subgraph))
    }

    /// Perform a series of actions, dispatching to [`Executor`]s when necessary.
    ///
    /// # Errors
    ///
    /// Will return Err if a Node cannot be run or dispatched.
    #[instrument(skip(self, actions), err)]
    pub async fn perform_actions(
        &self,
        workflow_run_id: Uuid,
        attempt: u32,
        mut actions: impl Stream<Item = miette::Result<Action>> + Unpin,
    ) -> miette::Result<()> {
        // Build a list of tasks to dispatch to Executors and immediately
        // process everything else.

        let mut exec_plans: HashMap<String, ActionPlan> = HashMap::new();
        for exec in self.executor_registry.keys() {
            exec_plans.insert(exec.clone(), ActionPlan::default());
        }
        let default_executor_name = &self.default_executor_name;

        //let mut plan = ActionPlan::default();
        let mut event_sender = self.event_sender.clone();
        while let Some(Action { loc, kind }) = actions.next().await.transpose()? {
            debug!("Running action at {loc}, kind: {kind:?}");
            match kind {
                ActionKind::PerformTask {
                    worker_name,
                    task_name,
                    inputs,
                    outputs,
                    resources,
                    environment,
                } => {
                    let executor = self.orchestrate(&resources, &environment);
                    exec_plans.get_mut(&executor).unwrap().tasks.push(TaskPlan {
                        worker_name,
                        task_name,
                        inputs: inputs.clone(),
                        output_storage_name: Some(self.default_storage_name.clone()),
                        resources,
                        environment,
                        outputs,
                        ..Default::default()
                    })
                }
                ActionKind::SetSwitching { cond } => {
                    exec_plans
                        .get_mut(default_executor_name)
                        .unwrap()
                        .switching
                        .push((loc, cond));
                }
                ActionKind::SetRunningLoop { index } => {
                    exec_plans
                        .get_mut(default_executor_name)
                        .unwrap()
                        .looping
                        .push((loc, index));
                }
                ActionKind::SetRunningMap { size } => {
                    exec_plans
                        .get_mut(default_executor_name)
                        .unwrap()
                        .mapping
                        .push((loc, size));
                }
                ActionKind::SetMapElemComplete { index, size } => {
                    let entry = exec_plans
                        .get_mut(default_executor_name)
                        .unwrap()
                        .map_elem_complete
                        .entry(loc)
                        .or_insert_with(|| BitVec::repeat(false, size));
                    entry.set(index, true);
                }
                ActionKind::SetComplete { outputs } => {
                    exec_plans
                        .get_mut(default_executor_name)
                        .unwrap()
                        .node_complete
                        .push((loc, outputs));
                }
                ActionKind::WorkflowFinished {} => {
                    exec_plans
                        .get_mut(default_executor_name)
                        .unwrap()
                        .workflow_complete = true;
                }
            }
        }

        if !exec_plans
            .get(default_executor_name)
            .unwrap()
            .node_complete
            .is_empty()
        {
            let (locs, outputs) = exec_plans
                .get(default_executor_name)
                .unwrap()
                .node_complete
                .clone()
                .into_iter()
                .unzip();
            send_complete(&mut event_sender, workflow_run_id, attempt, locs, outputs).await?;
        }

        for (loc, size) in exec_plans
            .get(default_executor_name)
            .unwrap()
            .mapping
            .clone()
        {
            send_running_map(&mut event_sender, workflow_run_id, attempt, loc, size).await?;
        }
        for (loc, bits) in exec_plans
            .get(default_executor_name)
            .unwrap()
            .map_elem_complete
            .clone()
        {
            send_map_elem_complete(&mut event_sender, workflow_run_id, attempt, loc, bits).await?;
        }
        for (loc, index) in exec_plans
            .get(default_executor_name)
            .unwrap()
            .looping
            .clone()
        {
            send_running_loop(&mut event_sender, workflow_run_id, attempt, loc, index).await?;
        }
        for (loc, cond) in exec_plans
            .get(default_executor_name)
            .unwrap()
            .switching
            .clone()
        {
            send_running_switching(&mut event_sender, workflow_run_id, attempt, loc, cond).await?;
        }

        for (executor_name, task_plans) in exec_plans {
            if task_plans.tasks.is_empty() {
                continue;
            }
            let executor = self
                .executor_registry
                .get(&executor_name)
                .ok_or_else(|| {
                    miette!(
                        "Could not find an executor with name '{executor_name}' in ExecutorRegistry"
                    )
                })
                .wrap_err("Could not run Task Nodes")?;

            executor
                .execute(task_plans.tasks)
                .await
                .wrap_err("Could not run Task Nodes")?;
        }

        Ok(())
    }

    fn orchestrate(
        &self,
        resources: &HashMap<String, Value>,
        environment: &HashMap<String, Value>,
    ) -> String {
        for (executor_name, executor) in self.executor_registry.iter() {
            if let Some(hpc_executor) = executor.as_ref().as_any().downcast_ref::<HPCExecutor>() {
                let hpc_resources = HPCResourceSpec::new(
                    resources.get("nodes").and_then(|v| v.as_u64()).unwrap_or(1) as u32,
                    resources
                        .get("cores_per_node")
                        .and_then(|v| v.as_u64())
                        .unwrap_or(1) as u32,
                    resources
                        .get("memory_per_node_gb")
                        .and_then(|v| v.as_u64())
                        .unwrap_or(1) as u32,
                    resources
                        .get("gpus_per_node")
                        .and_then(|v| v.as_u64())
                        .unwrap_or(0) as u32,
                );
                let hpc_environment = HPCEnvironmentSpec::new(
                    environment
                        .get("mpi_available")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false),
                );
                if hpc_executor.max_resources.satisfies(&hpc_resources)
                    && hpc_executor.environment.satisfies(&hpc_environment)
                {
                    return executor_name.clone();
                }
            }
        }
        self.default_executor_name.clone()
    }

    /// Listen to a combined stream events from the Orchestrator and the
    /// Executors in the registry.
    ///
    /// This method should only be called once and the stream will exist
    /// for the duration of the Workflow execution.
    ///
    /// # Errors
    ///
    /// Will return Err if the method has already been called.
    pub fn listen(&self) -> miette::Result<impl Stream<Item = RuntimeEvent> + use<>> {
        let orchestrator_events = {
            let mut receiver = self
                .event_receiver
                .try_lock()
                .map_err(|err| miette!("Failed to listen: {}", err))?;

            receiver.take().ok_or_else(|| {
                miette!("Failed to listen: Orchestrator is already being listened to.")
            })?
        };

        let mut streams = self
            .executor_registry
            .values()
            .map(|executor| {
                let stream = executor.listen()?;
                Ok(stream)
            })
            .collect::<miette::Result<Vec<BoxStream<RuntimeEvent>>>>()?;

        streams.push(orchestrator_events.boxed());
        Ok(select_all(streams))
    }
}

fn stream_action_result<'a>(res: miette::Result<Action>) -> BoxStream<'a, miette::Result<Action>> {
    stream::once(future::ready(res)).boxed()
}

fn stream_error<'a>(err: miette::Error) -> BoxStream<'a, miette::Result<Action>> {
    stream::once(future::err(err)).boxed()
}

#[instrument(
    skip(workflow_graph, node_states, parent_location, n),
    fields(location = %parent_location.with_node(n)),
    err,
)]
fn build_task_action(
    workflow_graph: Arc<WorkflowGraph>,
    node_states: &NodeStates,
    parent_location: &Location,
    n: NodeIndex,
    worker_name: &str,
    task_name: &str,
) -> miette::Result<Action> {
    let inputs = collect_inputs(&workflow_graph, node_states, parent_location, n)?;
    let outputs = workflow_graph.output_names(n)?.cloned().collect();

    Ok(Action {
        loc: parent_location.with_node(n),
        kind: ActionKind::PerformTask {
            worker_name: worker_name.to_string(),
            task_name: task_name.to_string(),
            inputs,
            outputs,
            resources: HashMap::new(),
            environment: HashMap::new(),
        },
    })
}

fn should_traverse_if_else_port(
    workflow_graph: &WorkflowGraph,
    node_cond: Option<bool>,
    p: PortIndex,
) -> bool {
    let port_name = workflow_graph
        .get_port_name(p)
        .expect("Failed to get port name");
    match &**port_name {
        "pred" => true,
        "if_true" => matches!(node_cond, Some(true)),
        "if_false" => matches!(node_cond, Some(false)),
        _ => panic!("Unexpected port name for `IfElse`"),
    }
}

#[instrument(
    skip_all,
    fields(location = %parent_location.with_node(n)),
    err,
)]
fn collect_inputs(
    workflow_graph: &WorkflowGraph,
    node_states: &NodeStates,
    parent_location: &Location,
    n: NodeIndex,
) -> miette::Result<HashMap<String, AssetSpec>> {
    let mut inputs = HashMap::new();
    for (i, o) in workflow_graph.input_links(n) {
        let input_name = workflow_graph.get_port_name(i.into())?;
        let output_name = workflow_graph.get_port_name(o.into())?;
        let linked_node = workflow_graph.port_node(o)?;
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
    use std::sync::Arc;

    use futures::StreamExt;
    use rstest::{fixture, rstest};
    use serde_json::json;

    use crate::{
        asset_storage::{FileAssetStorage, assert_registry_contains_values, test_storage_registry},
        builder::*,
        event::{NodeEvent, NodeStatus},
        executor::{
            hpc::HPCEnvironmentSpec, hpc::HPCExecutor, hpc::HPCResourceSpec,
            inmemory::InMemoryExecutor, interface::Executor, subprocess::SubprocessExecutor,
        },
        graph::LegacyWorkflowGraph,
        state::{inmemory::InMemoryWorkflowRunState, interface::NodeState},
    };

    use super::*;

    async fn test_executor_registry(
        asset_storage_registry: &AssetStorageRegistry,
    ) -> ExecutorRegistry {
        let mut executor_registry: HashMap<String, Box<dyn Executor>> = HashMap::new();

        executor_registry.insert(
            "memory".to_string(),
            Box::new(
                InMemoryExecutor::try_new(asset_storage_registry, "memory")
                    .await
                    .unwrap(),
            ),
        );
        executor_registry.insert(
            "subprocess".to_string(),
            Box::new(
                SubprocessExecutor::try_new(asset_storage_registry, "file", "file")
                    .await
                    .unwrap(),
            ),
        );

        let environment = HPCEnvironmentSpec::new(true);
        executor_registry.insert(
            "large".to_string(),
            Box::new(
                HPCExecutor::try_new(
                    asset_storage_registry,
                    "checkpoints",
                    "checkpoints",
                    HPCResourceSpec::new(2, 1, 4, 1),
                    environment.clone(),
                )
                .await
                .unwrap(),
            ),
        );
        executor_registry.insert(
            "small".to_string(),
            Box::new(
                HPCExecutor::try_new(
                    asset_storage_registry,
                    "checkpoints",
                    "checkpoints",
                    HPCResourceSpec::new(1, 1, 2, 1),
                    environment,
                )
                .await
                .unwrap(),
            ),
        );

        Arc::new(executor_registry)
    }

    #[fixture]
    fn one_input_one_output() -> WorkflowGraph {
        let mut wf = workflow(["out"]);
        let a = input(&mut wf, "a");
        let out = output(&wf, "out");

        link(&mut wf, a, out).expect("failed to link");

        wf
    }

    #[fixture]
    fn two_inputs_two_outputs() -> WorkflowGraph {
        let mut wf = workflow(["out1", "out2"]);
        let a = input(&mut wf, "a");
        let b = input(&mut wf, "b");
        let out1 = output(&wf, "out1");
        let out2 = output(&wf, "out2");

        link(&mut wf, a, out1).expect("failed to link");
        link(&mut wf, b, out2).expect("failed to link");

        wf
    }

    #[fixture]
    fn simple_task() -> WorkflowGraph {
        let mut wf = workflow(["out"]);
        let a = input(&mut wf, "a");
        let c = constant(&mut wf, 3).expect("failed to serialize");
        let add = task(&mut wf, "builtins", "iadd", ["a", "b"], ["value"]);
        let out = output(&wf, "out");

        link(&mut wf, a, (add, "a")).expect("failed to link");
        link(&mut wf, c, (add, "b")).expect("failed to link");
        link(&mut wf, (add, "value"), out).expect("failed to link");

        wf
    }

    #[fixture]
    fn doubler_plus() -> WorkflowGraph {
        let mut wf = workflow(["out"]);

        let a = input(&mut wf, "a");
        let b = input(&mut wf, "b");
        let two = constant(&mut wf, 2).expect("failed to serialize");

        let times = task(&mut wf, "builtins", "itimes", ["a", "b"], ["value"]);
        let add = task(&mut wf, "builtins", "iadd", ["a", "b"], ["value"]);

        link(&mut wf, a, (times, "a")).expect("failed to link");
        link(&mut wf, two, (times, "b")).expect("failed to link");
        link(&mut wf, (times, "value"), (add, "a")).expect("failed to link");
        link(&mut wf, b, (add, "b")).expect("failed to link");

        let out = output(&wf, "out");

        link(&mut wf, (add, "value"), out).expect("failed to link");

        wf
    }
    #[fixture]
    fn simple_eval() -> WorkflowGraph {
        let mut wf = workflow(["out"]);
        let a = input(&mut wf, "a");
        let sub = input(&mut wf, "subworkflow");
        let eval = eval(&mut wf, ["a"], ["out"]);
        let out = output(&wf, "out");

        link(&mut wf, a, (eval, "a")).expect("failed to link");
        link(&mut wf, sub, (eval, "graph")).expect("failed to link");
        link(&mut wf, (eval, "out"), out).expect("failed to link");

        wf
    }

    #[fixture]
    fn loop_body() -> WorkflowGraph {
        let mut wf = workflow(["loop_acc", "should_continue"]);
        let loop_acc_in = input(&mut wf, "loop_acc");
        let one = constant(&mut wf, 1).expect("failed to serialize");
        let limit = constant(&mut wf, 10).expect("failed to serialize");

        let add = task(&mut wf, "builtins", "iadd", ["a", "b"], ["value"]);
        let gt = task(&mut wf, "builtins", "igt", ["a", "b"], ["value"]);

        link(&mut wf, loop_acc_in, (add, "a")).expect("failed to link");
        link(&mut wf, one, (add, "b")).expect("failed to link");
        link(&mut wf, limit, (gt, "a")).expect("failed to link");
        link(&mut wf, (add, "value"), (gt, "b")).expect("failed to link");

        let loop_acc_out = output(&wf, "loop_acc");
        let should_continue = output(&wf, "should_continue");

        link(&mut wf, (add, "value"), loop_acc_out).expect("failed to link");
        link(&mut wf, (gt, "value"), should_continue).expect("failed to link");

        wf
    }

    #[fixture]
    fn simple_loop() -> WorkflowGraph {
        let mut wf = workflow(["out"]);
        let six = constant(&mut wf, 6).expect("failed to serialize");
        let loop_body = constant(&mut wf, loop_body()).expect("failed to serialize");
        let loop_node = loop_node(&mut wf, ["loop_acc"], ["loop_acc"]);
        let out = output(&wf, "out");

        link(&mut wf, six, (loop_node, "loop_acc")).expect("failed to link");
        link(&mut wf, loop_body, (loop_node, "graph")).expect("failed to link");
        link(&mut wf, (loop_node, "loop_acc"), out).expect("failed to link");

        wf
    }

    #[fixture]
    fn simple_map() -> WorkflowGraph {
        let mut wf = workflow(["out"]);

        let list = constant(&mut wf, (0..21).collect::<Vec<_>>()).expect("failed to serialize");
        let six = constant(&mut wf, 6).expect("failed to serialize");
        let doubler_plus = constant(&mut wf, doubler_plus()).expect("failed to serialize");
        let map = map_node(&mut wf, ["a"], ["b"], ["out"]);
        let out = output(&wf, "out");

        link(&mut wf, list, (map, "a")).expect("failed to link");
        link(&mut wf, six, (map, "b")).expect("failed to link");
        link(&mut wf, doubler_plus, (map, "graph")).expect("failed to link");
        link(&mut wf, (map, "out"), out).expect("failed to link");

        wf
    }

    #[fixture]
    fn simple_if_else() -> WorkflowGraph {
        let mut wf = workflow(["out"]);

        let one = constant(&mut wf, 1).expect("failed to serialize");
        let two = constant(&mut wf, 2).expect("failed to serialize");
        let pred = input(&mut wf, "pred");
        let if_else = if_else(&mut wf);
        let out = output(&wf, "out");

        link(&mut wf, one, (if_else, "if_true")).expect("failed to link");
        link(&mut wf, two, (if_else, "if_false")).expect("failed to link");
        link(&mut wf, pred, (if_else, "pred")).expect("failed to link");
        link(&mut wf, (if_else, "value"), out).expect("failed to link");

        wf
    }

    #[fixture]
    fn simple_eager_if_else() -> WorkflowGraph {
        let mut wf = workflow(["out"]);

        let one = constant(&mut wf, 1).expect("failed to serialize");
        let two = constant(&mut wf, 2).expect("failed to serialize");
        let pred = input(&mut wf, "pred");
        let eager_if_else = eager_if_else(&mut wf);
        let out = output(&wf, "out");

        link(&mut wf, one, (eager_if_else, "if_true")).expect("failed to link");
        link(&mut wf, two, (eager_if_else, "if_false")).expect("failed to link");
        link(&mut wf, pred, (eager_if_else, "pred")).expect("failed to link");
        link(&mut wf, (eager_if_else, "value"), out).expect("failed to link");

        wf
    }

    async fn next_actions(
        orchestrator: &Orchestrator,
        workflow_graph: &Arc<WorkflowGraph>,
        workflow_run_state: &Arc<dyn WorkflowRunState>,
        inputs: &HashMap<String, AssetSpec>,
    ) -> miette::Result<Vec<Action>> {
        let context = OrchestrationContext {
            parent_loc: Location::root(),
            graph_inputs: inputs.clone(),
            workflow_run_state: Arc::clone(workflow_run_state),
        };
        let actions = orchestrator
            .build_actions(context, Arc::clone(workflow_graph))
            .await?;
        let actions = actions
            .collect::<Vec<Result<_, _>>>()
            .await
            .into_iter()
            .collect::<Result<Vec<_>, _>>()?;
        Ok(actions)
    }

    // Test that we can plan a workflow with two input nodes.
    #[rstest]
    #[case::memory("memory")]
    #[case::file("file")]
    #[tokio::test]
    #[test_log::test]
    async fn plan_two_input_workflow(
        #[case] default_storage_name: &str,
        two_inputs_two_outputs: WorkflowGraph,
    ) -> miette::Result<()> {
        let (registry, input_sets, _dir) =
            test_storage_registry(vec![json!({"a": 1, "b": 4})], vec![]).await;
        let executor_registry = test_executor_registry(&registry).await;
        let orchestrator = Orchestrator::try_new(
            &registry,
            &executor_registry,
            default_storage_name,
            "memory",
        )
        .await?;

        let workflow_graph = Arc::new(two_inputs_two_outputs);

        let (workflow_run_state, _state_events) = InMemoryWorkflowRunState::test();
        let workflow_run_state: Arc<dyn WorkflowRunState> = Arc::new(workflow_run_state);
        let inputs = input_sets[0].clone();
        let actions =
            next_actions(&orchestrator, &workflow_graph, &workflow_run_state, &inputs).await?;

        assert_eq!(actions.len(), 2);
        assert!(matches!(actions[0].kind, ActionKind::SetComplete { .. }));
        assert!(matches!(actions[1].kind, ActionKind::SetComplete { .. }));

        Ok(())
    }

    // Test that we can plan the first actions of a workflow and run them.
    #[rstest]
    #[case::memory("memory")]
    #[case::file("file")]
    #[tokio::test]
    #[test_log::test]
    async fn plan_and_run_simple_io_workflow(
        #[case] default_storage_name: &str,
        one_input_one_output: WorkflowGraph,
    ) -> miette::Result<()> {
        let (registry, input_sets, _dir) =
            test_storage_registry(vec![json!({"a": 1})], vec![]).await;
        let executor_registry = test_executor_registry(&registry).await;
        let orchestrator = Orchestrator::try_new(
            &registry,
            &executor_registry,
            default_storage_name,
            "memory",
        )
        .await?;
        let mut stream = orchestrator.listen()?;
        let workflow_graph = Arc::new(one_input_one_output);

        let inputs = input_sets[0].clone();
        let (workflow_run_state, _state_events) = InMemoryWorkflowRunState::test();
        let workflow_run_state: Arc<dyn WorkflowRunState> = Arc::new(workflow_run_state);
        let actions =
            next_actions(&orchestrator, &workflow_graph, &workflow_run_state, &inputs).await?;

        assert_eq!(actions.len(), 1);
        assert_eq!(actions[0].loc, Location::new("N1")?);
        assert!(matches!(actions[0].kind, ActionKind::SetComplete { .. }));

        orchestrator
            .perform_actions(Uuid::nil(), 0, stream::iter(actions.into_iter().map(Ok)))
            .await?;
        let input_complete_event = stream.next().await.unwrap();
        let input_complete_outputs = input_complete_event.clone().outputs();
        assert_registry_contains_values(
            &registry,
            "memory", // Asset is not modified so it remains in memory.
            &input_complete_outputs[0],
            json!({"a": 1}),
        )
        .await;

        let input_complete_event = match input_complete_event {
            RuntimeEvent::WorkflowRun { event, .. } => event,
        };

        workflow_run_state.write(input_complete_event).await?;
        let actions =
            next_actions(&orchestrator, &workflow_graph, &workflow_run_state, &inputs).await?;

        assert_eq!(actions.len(), 2);
        assert_eq!(
            actions[0].loc,
            Location::from_node_index_iter([workflow_graph.output_idx()])
        );
        assert!(matches!(actions[0].kind, ActionKind::SetComplete { .. }));
        assert_eq!(
            actions[1].loc,
            Location::from_node_index_iter([workflow_graph.output_idx()])
        );
        assert!(matches!(actions[1].kind, ActionKind::WorkflowFinished {}));

        orchestrator
            .perform_actions(
                workflow_run_state.run_id(),
                workflow_run_state.attempt(),
                stream::iter(actions.into_iter().map(Ok)),
            )
            .await?;
        let output_complete_event = stream.next().await.unwrap();
        let output_complete_outputs = output_complete_event.outputs();
        assert_registry_contains_values(
            &registry,
            "memory", // Asset is not modified so it remains in memory.
            &output_complete_outputs[0],
            json!({"out": 1}),
        )
        .await;

        Ok(())
    }

    // Test that we can plan the first actions of a workflow and run them.
    #[rstest]
    #[case::memory("memory")]
    #[case::file("file")]
    #[tokio::test]
    #[test_log::test]
    async fn plan_and_run_simple_eval_workflow(
        #[case] default_storage_name: &str,
        one_input_one_output: WorkflowGraph,
        simple_eval: WorkflowGraph,
    ) -> miette::Result<()> {
        let (registry, input_sets, _dir) = test_storage_registry(
            vec![json!({"a": 1, "subworkflow": one_input_one_output})],
            vec![],
        )
        .await;
        let executor_registry = test_executor_registry(&registry).await;
        let orchestrator = Orchestrator::try_new(
            &registry,
            &executor_registry,
            default_storage_name,
            "memory",
        )
        .await?;
        let mut stream = orchestrator.listen()?;

        let workflow_graph = Arc::new(simple_eval);

        let (workflow_run_state, _state_events) = InMemoryWorkflowRunState::test();
        let workflow_run_state: Arc<dyn WorkflowRunState> = Arc::new(workflow_run_state);
        let inputs = input_sets[0].clone();
        let actions =
            next_actions(&orchestrator, &workflow_graph, &workflow_run_state, &inputs).await?;

        assert_eq!(actions.len(), 2);
        assert_eq!(actions[0].loc, Location::new("N2")?);
        assert!(matches!(actions[0].kind, ActionKind::SetComplete { .. }));
        assert_eq!(actions[1].loc, Location::new("N1")?);
        assert!(matches!(actions[1].kind, ActionKind::SetComplete { .. }));

        orchestrator
            .perform_actions(Uuid::nil(), 0, stream::iter(actions.into_iter().map(Ok)))
            .await?;

        let inputs_complete_event = stream.next().await.unwrap();
        let inputs_complete_outputs = inputs_complete_event.clone().outputs();
        assert_registry_contains_values(
            &registry,
            "memory", // Asset is not modified so it remains in memory.
            &inputs_complete_outputs[0],
            json!({"subworkflow": one_input_one_output}),
        )
        .await;
        assert_registry_contains_values(
            &registry,
            "memory", // Asset is not modified so it remains in memory.
            &inputs_complete_outputs[1],
            json!({"a": 1}),
        )
        .await;

        let inputs_complete_event = match inputs_complete_event {
            RuntimeEvent::WorkflowRun { event, .. } => event,
        };

        workflow_run_state.write(inputs_complete_event).await?;

        let actions =
            next_actions(&orchestrator, &workflow_graph, &workflow_run_state, &inputs).await?;

        assert_eq!(actions.len(), 1);
        assert_eq!(actions[0].loc, Location::new("N3.N1")?);
        assert!(matches!(actions[0].kind, ActionKind::SetComplete { .. }));

        orchestrator
            .perform_actions(
                workflow_run_state.run_id(),
                workflow_run_state.attempt(),
                stream::iter(actions.into_iter().map(Ok)),
            )
            .await?;
        let inner_inputs_complete_event = stream.next().await.unwrap();
        let inner_inputs_complete_outputs = inner_inputs_complete_event.clone().outputs();
        assert_registry_contains_values(
            &registry,
            "memory", // Asset is not modified so it remains in memory.
            &inner_inputs_complete_outputs[0],
            json!({"a": 1}),
        )
        .await;

        let inner_inputs_complete_event = match inner_inputs_complete_event {
            RuntimeEvent::WorkflowRun { event, .. } => event,
        };

        workflow_run_state
            .write(inner_inputs_complete_event)
            .await?;
        let actions =
            next_actions(&orchestrator, &workflow_graph, &workflow_run_state, &inputs).await?;

        orchestrator
            .perform_actions(
                workflow_run_state.run_id(),
                workflow_run_state.attempt(),
                stream::iter(actions.into_iter().map(Ok)),
            )
            .await?;
        let inner_output_complete_event = stream.next().await.unwrap();
        let inner_output_complete_outputs = inner_output_complete_event.clone().outputs();
        assert_registry_contains_values(
            &registry,
            "memory", // Asset is not modified so it remains in memory.
            &inner_output_complete_outputs[0],
            json!({"out": 1}),
        )
        .await;

        let inner_output_complete_event = match inner_output_complete_event {
            RuntimeEvent::WorkflowRun { event, .. } => event,
        };

        let eval_complete_event = WorkflowRunEvent::NodeEvent(NodeEvent {
            locs: vec![Location::new("N3")?],
            status: NodeStatus::Complete {
                outputs: inner_output_complete_outputs,
            },
        });
        workflow_run_state
            .write(inner_output_complete_event)
            .await?;
        workflow_run_state.write(eval_complete_event).await?;
        let actions =
            next_actions(&orchestrator, &workflow_graph, &workflow_run_state, &inputs).await?;

        orchestrator
            .perform_actions(Uuid::nil(), 0, stream::iter(actions.into_iter().map(Ok)))
            .await?;
        let output_complete_event = stream.next().await.unwrap();
        let output_complete_outputs = output_complete_event.outputs();
        assert_registry_contains_values(
            &registry,
            "memory", // Asset is not modified so it remains in memory.
            &output_complete_outputs[0],
            json!({"out": 1}),
        )
        .await;

        Ok(())
    }

    // Test that we can plan the first actions of a workflow and run them.
    #[rstest]
    #[case::memory("memory")]
    #[case::file("file")]
    #[tokio::test]
    #[test_log::test]
    async fn run_simple_task_workflow(
        #[case] default_storage_name: &str,
        simple_task: WorkflowGraph,
    ) -> miette::Result<()> {
        let workflow_graph = Arc::new(simple_task);

        let (registry, input_sets, _dir) =
            test_storage_registry(vec![json!({"a": 1})], vec![]).await;
        let executor_registry = test_executor_registry(&registry).await;
        let orchestrator = Orchestrator::try_new(
            &registry,
            &executor_registry,
            default_storage_name,
            "memory",
        )
        .await?;

        let (workflow_run_state, mut state_recv) = InMemoryWorkflowRunState::test();
        let workflow_run_state: Arc<dyn WorkflowRunState> = Arc::new(workflow_run_state);
        let inputs = input_sets[0].clone();
        let context = OrchestrationContext {
            parent_loc: Location::root(),
            graph_inputs: inputs.clone(),
            workflow_run_state: Arc::clone(&workflow_run_state),
        };
        let mut stream = orchestrator.listen()?;
        let state = Arc::clone(&workflow_run_state);

        tokio::spawn(async move {
            while let Some(event) = stream.next().await {
                match event {
                    RuntimeEvent::WorkflowRun { event, .. } => {
                        state.write(event).await.unwrap();
                    }
                }
            }
        });
        loop {
            {
                let updated = state_recv.borrow_and_update();
                if updated.active_runs.is_empty() {
                    break;
                }
            }

            let actions = orchestrator
                .build_actions(context.clone(), workflow_graph.clone())
                .await?;

            orchestrator
                .perform_actions(
                    workflow_run_state.run_id(),
                    workflow_run_state.attempt(),
                    actions,
                )
                .await?;
            state_recv.changed().await.into_diagnostic()?;
        }

        let a_input_state = workflow_run_state.read(&Location::new("N1")?).await?;

        assert!(matches!(
            a_input_state,
            NodeState {
                complete_time: Some(..),
                outputs: Some(..),
                ..
            }
        ));

        let subworkflow_input_state = workflow_run_state.read(&Location::new("N2")?).await?;

        assert!(matches!(
            subworkflow_input_state,
            NodeState {
                complete_time: Some(..),
                outputs: Some(..),
                ..
            }
        ));

        let output_state = workflow_run_state.read(&Location::new("N0")?).await?;

        let outputs = output_state.outputs.expect("no outputs found");

        assert_registry_contains_values(
            &registry,
            default_storage_name,
            &outputs,
            json!({"out": 4}),
        )
        .await;

        Ok(())
    }

    // Test that we can plan the first actions of a workflow and run them.
    #[rstest]
    #[case::memory("memory")]
    #[case::file("file")]
    #[tokio::test]
    #[test_log::test]
    async fn run_simple_eval_workflow(
        #[case] default_storage_name: &str,
        one_input_one_output: WorkflowGraph,
        simple_eval: WorkflowGraph,
    ) -> miette::Result<()> {
        let workflow_graph = Arc::new(simple_eval);

        let (registry, input_sets, _dir) = test_storage_registry(
            vec![json!({"a": 1, "subworkflow": one_input_one_output})],
            vec![],
        )
        .await;
        let executor_registry = test_executor_registry(&registry).await;
        let orchestrator = Orchestrator::try_new(
            &registry,
            &executor_registry,
            default_storage_name,
            "memory",
        )
        .await?;

        let (workflow_run_state, mut state_recv) = InMemoryWorkflowRunState::test();
        let workflow_run_state: Arc<dyn WorkflowRunState> = Arc::new(workflow_run_state);
        let inputs = input_sets[0].clone();
        let context = OrchestrationContext {
            parent_loc: Location::root(),
            graph_inputs: inputs.clone(),
            workflow_run_state: Arc::clone(&workflow_run_state),
        };
        let mut stream = orchestrator.listen()?;
        let state = Arc::clone(&workflow_run_state);

        tokio::spawn(async move {
            while let Some(event) = stream.next().await {
                match event {
                    RuntimeEvent::WorkflowRun { event, .. } => {
                        state.write(event).await.unwrap();
                    }
                }
            }
        });

        loop {
            {
                let updated = state_recv.borrow_and_update();
                if updated.active_runs.is_empty() {
                    break;
                }
            }

            let actions = orchestrator
                .build_actions(context.clone(), workflow_graph.clone())
                .await?;
            orchestrator
                .perform_actions(
                    workflow_run_state.run_id(),
                    workflow_run_state.attempt(),
                    actions,
                )
                .await?;
            state_recv.changed().await.into_diagnostic()?;
        }

        let a_input_state = workflow_run_state.read(&Location::new("N1")?).await?;

        assert!(matches!(
            a_input_state,
            NodeState {
                complete_time: Some(..),
                outputs: Some(..),
                ..
            }
        ));

        let subworkflow_input_state = workflow_run_state.read(&Location::new("N2")?).await?;

        assert!(matches!(
            subworkflow_input_state,
            NodeState {
                complete_time: Some(..),
                outputs: Some(..),
                ..
            }
        ));

        let output_state = workflow_run_state
            .read(&Location::from_node_index_iter([
                workflow_graph.output_idx()
            ]))
            .await?;

        let outputs = output_state.outputs.expect("no outputs found");

        assert_registry_contains_values(
            &registry,
            "memory", // Asset is not modified so it remains in memory.
            &outputs,
            json!({"out": 1}),
        )
        .await;

        Ok(())
    }

    #[rstest]
    #[case::one_input_one_output_3(one_input_one_output(), json!({"a": 3}), json!({"out": 3}))]
    #[case::one_input_one_output_6(one_input_one_output(), json!({"a": 6}), json!({"out": 6}))]
    #[case::two_inputs_two_outputs(two_inputs_two_outputs(), json!({"a": 1, "b": 3}), json!({"out1": 1, "out2": 3}))]
    #[case::simple_task_1(simple_task(), json!({"a": 1}), json!({"out": 4}))]
    #[case::simple_task_5(simple_task(), json!({"a": 5}), json!({"out": 8}))]
    #[case::simple_eval(simple_eval(), json!({"a": 3, "subworkflow": one_input_one_output()}), json!({"out": 3}))]
    #[case::simple_loop(simple_loop(), json!({}), json!({"out": 10}))]
    #[case::simple_map(simple_map(), json!({}), json!({"out": (0..21).map(|x| x * 2 + 6).collect::<Vec<_>>()}))]
    #[case::simple_if_else_true(simple_if_else(), json!({"pred": true}), json!({"out": 1}))]
    #[case::simple_if_else_false(simple_if_else(), json!({"pred": false}), json!({"out": 2}))]
    #[case::simple_eager_if_else_true(simple_eager_if_else(), json!({"pred": true}), json!({"out": 1}))]
    #[case::simple_eager_if_else_false(simple_eager_if_else(), json!({"pred": false}), json!({"out": 2}))]
    #[tokio::test]
    #[test_log::test]
    async fn run_workflows(
        #[case] workflow_graph: WorkflowGraph,
        #[case] inputs: serde_json::Value,
        #[case] expected_outputs: serde_json::Value,
    ) -> miette::Result<()> {
        let default_storage_name = "memory";
        let workflow_graph = Arc::new(workflow_graph);

        let (registry, input_sets, _dir) = test_storage_registry(vec![inputs], vec![]).await;
        let executor_registry = test_executor_registry(&registry).await;
        let orchestrator = Orchestrator::try_new(
            &registry,
            &executor_registry,
            default_storage_name,
            "memory",
        )
        .await?;

        let (workflow_run_state, mut state_recv) = InMemoryWorkflowRunState::test();
        let workflow_run_state: Arc<dyn WorkflowRunState> = Arc::new(workflow_run_state);
        let inputs = input_sets[0].clone();
        let context = OrchestrationContext {
            parent_loc: Location::root(),
            graph_inputs: inputs.clone(),
            workflow_run_state: Arc::clone(&workflow_run_state),
        };
        let mut stream = orchestrator.listen()?;
        let state = Arc::clone(&workflow_run_state);

        tokio::spawn(async move {
            while let Some(event) = stream.next().await {
                match event {
                    RuntimeEvent::WorkflowRun { event, .. } => {
                        state.write(event).await.unwrap();
                    }
                }
            }
        });

        loop {
            {
                let updated = state_recv.borrow_and_update();
                if updated.active_runs.is_empty() {
                    break;
                }
            }

            let actions = orchestrator
                .build_actions(context.clone(), workflow_graph.clone())
                .await?;
            orchestrator
                .perform_actions(
                    workflow_run_state.run_id(),
                    workflow_run_state.attempt(),
                    actions,
                )
                .await?;
            state_recv.changed().await.into_diagnostic()?;
        }

        let output_state = workflow_run_state
            .read(&Location::from_node_index_iter([
                workflow_graph.output_idx()
            ]))
            .await?;

        let outputs = output_state.outputs.expect("no outputs found");

        assert_registry_contains_values(
            &registry,
            &orchestrator.default_storage_name,
            &outputs,
            expected_outputs,
        )
        .await;

        Ok(())
    }

    // Test that we can plan the first actions of a workflow and run them.
    #[rstest]
    #[case::memory("memory")]
    #[case::file("file")]
    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    async fn run_serialized_workflow(#[case] default_storage_name: &str) -> miette::Result<()> {
        let serialized_graph = include_str!("../tests/cli/data/sample_graph");
        let graph: LegacyWorkflowGraph =
            serde_json::from_str(serialized_graph).into_diagnostic()?;
        let workflow_graph = Arc::new(graph.to_workflow_graph().unwrap());

        let (registry, _input_sets, _dir) = test_storage_registry(vec![], vec![]).await;
        let executor_registry = test_executor_registry(&registry).await;
        let orchestrator = Orchestrator::try_new(
            &registry,
            &executor_registry,
            default_storage_name,
            "memory",
        )
        .await?;

        let (workflow_run_state, mut state_recv) = InMemoryWorkflowRunState::test();
        let workflow_run_state: Arc<dyn WorkflowRunState> = Arc::new(workflow_run_state);
        let context = OrchestrationContext {
            parent_loc: Location::root(),
            graph_inputs: HashMap::new(),
            workflow_run_state: Arc::clone(&workflow_run_state),
        };
        let mut stream = orchestrator.listen()?;
        let state = Arc::clone(&workflow_run_state);

        tokio::spawn(async move {
            while let Some(event) = stream.next().await {
                match event {
                    RuntimeEvent::WorkflowRun { event, .. } => {
                        state.write(event).await.unwrap();
                    }
                }
            }
        });

        loop {
            {
                let updated = state_recv.borrow_and_update();
                if updated.active_runs.is_empty() {
                    break;
                }
            }

            let actions = orchestrator
                .build_actions(context.clone(), workflow_graph.clone())
                .await?;
            orchestrator
                .perform_actions(
                    workflow_run_state.run_id(),
                    workflow_run_state.attempt(),
                    actions,
                )
                .await?;
            state_recv.changed().await.into_diagnostic()?;
        }

        let output_state = workflow_run_state
            .read(&Location::from_node_index_iter([
                workflow_graph.output_idx()
            ]))
            .await?;

        let outputs = output_state.outputs.expect("no outputs found");

        assert_registry_contains_values(
            &registry,
            &orchestrator.default_storage_name,
            &outputs,
            json!({"simple_eval_output": 12}),
        )
        .await;

        Ok(())
    }

    #[rstest]
    #[tokio::test]
    async fn do_resource_orchestration() -> miette::Result<()> {
        // Setup Orchestrator and registry
        let file_storage = FileAssetStorage::new(std::path::Path::new(
            "/Users/philipp.seitz/.tierkreis/checkpoints/00000000-0000-0000-0000-000000000016/",
        ));
        let (registry, input_sets, _dir) =
            test_storage_registry(vec![json!({"value": "Test"})], vec![]).await;
        registry
            .write()
            .await
            .insert("checkpoints".to_string(), Box::new(file_storage));
        let executor_registry = test_executor_registry(&registry).await;
        let orchestrator =
            Orchestrator::try_new(&registry, &executor_registry, "memory", "memory").await?;
        let stream = orchestrator.listen()?;

        // new setup
        let (workflow_run_state, _state_recv) = InMemoryWorkflowRunState::test();
        let workflow_run_state = Arc::new(workflow_run_state);
        let action = Action {
            loc: Location::new("N1")?,
            kind: ActionKind::PerformTask {
                worker_name: "mpi_worker".to_string(),
                task_name: "mpi_rank_info_with_input".to_string(),
                resources: HashMap::from([("nodes".to_string(), json!(2))]),
                environment: HashMap::from([("mpi_available".to_string(), json!(true))]),
                inputs: input_sets[0].clone(),
                outputs: HashSet::from(["out".to_string()]),
            },
        };
        orchestrator
            .perform_actions(
                workflow_run_state.run_id(),
                workflow_run_state.attempt(),
                stream::iter(vec![Ok(action)]),
            )
            .await?;
        let events = stream.take(2).collect::<Vec<_>>().await;
        assert_eq!(events.len(), 2);
        assert!(matches!(
            events[0],
            RuntimeEvent::WorkflowRun {
                event: WorkflowRunEvent::NodeEvent(NodeEvent {
                    status: NodeStatus::Running { .. },
                    ..
                }),
                ..
            }
        ));
        dbg!(events.clone());
        assert_registry_contains_values(
            &registry,
            "memory",
            &events[1].clone().outputs()[0],
            json!({"value": "Rank 0 out of 2 on c1 with value Test.\nRank 1 out of 2 on c2 with value Test."}),
        )
        .await;

        Ok(())
    }
}
