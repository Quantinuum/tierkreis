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
    Stream, StreamExt, TryFutureExt,
    channel::mpsc,
    future,
    stream::{self, BoxStream, LocalBoxStream, select_all},
};
use miette::{Context, IntoDiagnostic, miette};
use portgraph::{NodeIndex, PortIndex};
use tracing::instrument;

use crate::{
    asset_storage::{
        AssetStorageRegistry, fold_assets, interface::AssetSpec, load_asset, save_asset,
        transfer_assets, unfold_asset,
    },
    event::{
        Event, EventReceiver, EventSender, NodeEvent, send_complete, send_map_elem_complete,
        send_running_loop, send_running_map, send_running_switching, send_workflow_run_complete,
    },
    executor::{ExecutorRegistry, interface::TaskPlan},
    graph::{LegacyWorkflowGraph, NodeDefinition, WorkflowGraph},
    location::Location,
    state::{WorkflowState, interface::NodeState},
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
        /// The input assets for the Task.
        inputs: HashMap<String, AssetSpec>,
    },
    /// Mark the node as switching with a particular value.
    SetSwitching {
        /// The value to mark the node with.
        cond: bool,
    },
    SetRunningLoop {
        index: u32,
    },
    SetRunningMap {
        size: u32,
    },
    SetMapElemComplete {
        index: u32,
    },
    /// Mark the node as complete with outputs.
    SetComplete {
        /// The output values for the node.
        outputs: HashMap<String, AssetSpec>,
    },
    WorkflowFinished {},
}

/// The context state for the orchestration
#[derive(Debug, Clone)]
pub struct OrchestrationContext {
    subworkflow_context: Location,
    graph_inputs: HashMap<String, AssetSpec>,
    workflow_state: Arc<dyn WorkflowState>,
}

impl OrchestrationContext {
    /// Construct a new [`OrchestrationContext`] at the root Location.
    pub fn new(
        workflow_state: &Arc<dyn WorkflowState>,
        inputs: HashMap<String, AssetSpec>,
    ) -> Self {
        Self {
            subworkflow_context: Location::root(),
            graph_inputs: inputs,
            workflow_state: Arc::clone(workflow_state),
        }
    }
}

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
    pub fn try_new(
        asset_storage_registry: &AssetStorageRegistry,
        executor_registry: &ExecutorRegistry,
        default_storage_name: &str,
        default_executor_name: &str,
    ) -> miette::Result<Self> {
        let (sender, receiver) = mpsc::channel(128);

        let asset_storage_registry_lock = asset_storage_registry
            .read()
            .map_err(|err| miette!("Failed to lock AssetStorageRegistry for reading: {err}"))?;
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
    #[instrument(skip(self, context, workflow_graph), err)]
    pub async fn build_actions(
        &self,
        context: OrchestrationContext,
        workflow_graph: Arc<WorkflowGraph>,
    ) -> miette::Result<LocalBoxStream<'_, miette::Result<Action>>> {
        let node_states = Self::collect_node_states(&context, &workflow_graph).await?;

        let output_state = node_states
            .get(&workflow_graph.output_idx())
            .ok_or_else(|| miette!("No output node state found"))?;
        if output_state.1.scheduled_time.is_some() {
            // Output is already scheduled, no actions to perform.
            return Ok(stream::empty().boxed());
        }

        let ready_nodes: Vec<_> = Self::find_ready_nodes(&workflow_graph, &node_states).collect();
        // Mark all ready nodes as scheduled.
        Self::mark_nodes_scheduled(&context, ready_nodes.iter()).await?;

        let parent_location = context.subworkflow_context.clone();
        let graph_inputs = Arc::new(context.graph_inputs.clone());
        let workflow_state = Arc::clone(&context.workflow_state);
        let node_states = Arc::new(node_states);

        Ok(stream::iter(ready_nodes)
            .flat_map_unordered(None, move |n| -> LocalBoxStream<miette::Result<Action>> {
                let node_states = Arc::clone(&node_states);
                let Some((definition, state)) = node_states.get(&n) else {
                    return stream_error(miette!("Could not find node definition/state"));
                };

                let loc = parent_location.with_node(n);
                match definition {
                    NodeDefinition::Input { name } => stream_action_result(
                        self.build_input_action(graph_inputs.clone(), loc, name),
                    ),
                    NodeDefinition::Const { value } => {
                        stream_action_result(self.build_const_action(loc, value.clone()))
                    }
                    NodeDefinition::Output {} => self
                        .build_output_actions(workflow_graph.clone(), node_states.clone(), n, loc)
                        .try_flatten_stream()
                        .boxed_local(),
                    NodeDefinition::Task {
                        worker_name,
                        task_name,
                    } => stream_action_result(build_task_action(
                        workflow_graph.clone(),
                        node_states.clone(),
                        n,
                        loc,
                        worker_name,
                        task_name,
                    )),
                    NodeDefinition::Eval {} => self
                        .build_eval_actions(
                            workflow_graph.clone(),
                            workflow_state.clone(),
                            node_states.clone(),
                            n,
                            loc,
                        )
                        .try_flatten_stream()
                        .boxed_local(),
                    NodeDefinition::Loop {} => self
                        .build_loop_actions(
                            workflow_graph.clone(),
                            workflow_state.clone(),
                            node_states.clone(),
                            n,
                            loc,
                            state.loop_index,
                        )
                        .try_flatten_stream()
                        .boxed_local(),
                    NodeDefinition::Map { mapped_ports } => self
                        .build_map_actions(
                            workflow_graph.clone(),
                            workflow_state.clone(),
                            node_states.clone(),
                            mapped_ports.clone(),
                            n,
                            loc,
                            state.map_completed.clone(),
                        )
                        .try_flatten_stream()
                        .boxed_local(),
                    // Eager and Lazy If else are controlled by the ready node checks
                    NodeDefinition::IfElse {} => stream_action_result(self.build_if_else_action(
                        workflow_graph.clone(),
                        node_states.clone(),
                        n,
                        loc,
                        state.cond,
                    )),
                    NodeDefinition::EagerIfElse {} => {
                        stream_action_result(self.build_eager_if_else_action(
                            workflow_graph.clone(),
                            node_states.clone(),
                            n,
                            loc,
                        ))
                    }
                }
            })
            .boxed_local())
    }

    async fn collect_node_states(
        context: &OrchestrationContext,
        workflow_graph: &Arc<WorkflowGraph>,
    ) -> miette::Result<HashMap<NodeIndex, (NodeDefinition, NodeState)>> {
        let mut node_states = HashMap::new();
        for node_id in workflow_graph.node_ids() {
            let location = context.subworkflow_context.with_node(node_id);
            let node_definition = workflow_graph
                .node_definition(node_id)
                .ok_or_else(|| miette!("Node definition not found"))?;
            let node_state = context.workflow_state.read(&location).await?;
            if let Some(error_msg) = node_state.error {
                return Err(miette!("Workflow ended with error: {error_msg}"));
            }

            node_states.insert(node_id, (node_definition.clone(), node_state));
        }
        Ok(node_states)
    }

    /// Find nodes which are ready for execution, mark them as scheduled then return them.
    fn find_ready_nodes<'a>(
        workflow_graph: &'a WorkflowGraph,
        node_states: &'a HashMap<NodeIndex, (NodeDefinition, NodeState)>,
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
                    let (definition, state) = node_states
                        .get(&n)
                        .expect("Node definition/state not found");

                    let has_outputs = state.outputs.is_some();
                    let not_already_scheduled = state.scheduled_time.is_none();
                    // Always traverse control nodes.
                    let is_control_flow = matches!(
                        definition,
                        NodeDefinition::Eval {}
                            | NodeDefinition::Loop {}
                            | NodeDefinition::Map { .. }
                            | NodeDefinition::IfElse {}
                    );
                    !has_outputs && (not_already_scheduled || is_control_flow)
                },
                // Returns true if a port should be traversed.
                |n, p| {
                    let (definition, state) = node_states
                        .get(&n)
                        .expect("Node definition/state not found");
                    if matches!(definition, NodeDefinition::IfElse {}) {
                        should_traverse_if_else_port(workflow_graph, state.cond, p)
                    } else {
                        true
                    }
                },
            )
            // Of the nodes that have not yet run, find the nodes that can run.
            // (Because their inputs are ready or otherwise.)
            .filter(|n| {
                let (definition, state) =
                    node_states.get(n).expect("Node definition/state not found");
                if matches!(definition, NodeDefinition::IfElse {}) {
                    Self::if_else_ready(workflow_graph, node_states, *n, state)
                } else {
                    workflow_graph.all_inputs(*n, |incoming| {
                        node_states
                            .get(&incoming)
                            .is_some_and(|(_, state)| state.outputs.is_some())
                    })
                }
            })
    }

    // Returns true if an `IfElse` node is runnable.
    fn if_else_ready(
        workflow_graph: &WorkflowGraph,
        node_states: &HashMap<NodeIndex, (NodeDefinition, NodeState)>,
        n: NodeIndex,
        state: &NodeState,
    ) -> bool {
        match state.cond {
            None => Self::port_has_input(workflow_graph, node_states, n, "pred")
                .expect("No `pred` port on `IfElse` node"),
            Some(true) => Self::port_has_input(workflow_graph, node_states, n, "if_true")
                .expect("No `if_true` port on `IfElse` node"),
            Some(false) => Self::port_has_input(workflow_graph, node_states, n, "if_false")
                .expect("No `if_false` port on `IfElse` node"),
        }
    }

    // Returns true if a port on a node's input is available.
    fn port_has_input(
        workflow_graph: &WorkflowGraph,
        node_states: &HashMap<NodeIndex, (NodeDefinition, NodeState)>,
        n: NodeIndex,
        port_name: &str,
    ) -> miette::Result<bool> {
        let (connected_node, _) = workflow_graph.connected_input_by_port_name(n, port_name)?;

        Ok(node_states
            .get(&connected_node)
            .is_some_and(|(_, state)| state.outputs.is_some()))
    }

    async fn mark_nodes_scheduled(
        context: &OrchestrationContext,
        nodes: impl Iterator<Item = &NodeIndex>,
    ) -> miette::Result<()> {
        for node in nodes {
            context
                .workflow_state
                .write(Event::Node(NodeEvent {
                    loc: context.subworkflow_context.with_node(*node),
                    status: crate::event::NodeStatus::Scheduled {},
                }))
                .await?;
        }

        Ok(())
    }

    #[instrument(skip(self, workflow_graph, node_states), err)]
    fn build_if_else_action(
        &self,
        workflow_graph: Arc<WorkflowGraph>,
        node_states: Arc<HashMap<NodeIndex, (NodeDefinition, NodeState)>>,
        n: NodeIndex,
        loc: Location,
        cond: Option<bool>,
    ) -> miette::Result<Action> {
        match cond {
            None => {
                let (pred_node, connected_port) =
                    workflow_graph.connected_input_by_port_name(n, "pred")?;
                let (_, pred_state) = node_states
                    .get(&pred_node)
                    .ok_or_else(|| miette!("Cannot find state for `pred` node"))?;
                let connected_port_name = workflow_graph.get_port_name(connected_port)?;

                let pred_bytes = load_asset(
                    &self.asset_storage_registry,
                    pred_state
                        .outputs
                        .as_ref()
                        .ok_or_else(|| miette!("`pred` node has no outputs"))?,
                    connected_port_name,
                )?;

                Ok(Action {
                    loc,
                    kind: ActionKind::SetSwitching {
                        cond: pred_bytes == b"true",
                    },
                })
            }
            Some(true) => {
                Self::build_branch_action(&workflow_graph, &node_states, n, loc, "if_true")
            }
            Some(false) => {
                Self::build_branch_action(&workflow_graph, &node_states, n, loc, "if_false")
            }
        }
    }

    fn build_branch_action(
        workflow_graph: &Arc<WorkflowGraph>,
        node_states: &Arc<HashMap<NodeIndex, (NodeDefinition, NodeState)>>,
        n: NodeIndex,
        loc: Location,
        port_name: &str,
    ) -> miette::Result<Action> {
        let (connected_node, connected_port) =
            workflow_graph.connected_input_by_port_name(n, port_name)?;
        let (_, node_state) = node_states
            .get(&connected_node)
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
            loc,
            kind: ActionKind::SetComplete { outputs },
        })
    }

    #[instrument(skip(self, workflow_graph, node_states), err)]
    fn build_eager_if_else_action(
        &self,
        workflow_graph: Arc<WorkflowGraph>,
        node_states: Arc<HashMap<NodeIndex, (NodeDefinition, NodeState)>>,
        n: NodeIndex,
        loc: Location,
    ) -> miette::Result<Action> {
        let mut inputs = collect_inputs(&workflow_graph, &node_states, n)?;

        let pred_bytes = load_asset(&self.asset_storage_registry, &inputs, "pred")?;
        let mut outputs = HashMap::new();

        if pred_bytes == b"true" {
            let value = inputs.remove("if_true").unwrap();
            outputs.insert("value".to_string(), value);
        } else {
            let value = inputs.remove("if_false").unwrap();
            outputs.insert("value".to_string(), value);
        }

        Ok(Action {
            loc,
            kind: ActionKind::SetComplete { outputs },
        })
    }

    #[instrument(skip(self), err)]
    fn build_input_action(
        &self,
        graph_inputs: Arc<HashMap<String, AssetSpec>>,
        loc: Location,
        name: &String,
    ) -> miette::Result<Action> {
        let outputs: HashMap<String, AssetSpec> = graph_inputs
            .iter()
            .filter(|(k, _)| k == &name)
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        // TODO: It's a little bit unclear if this is desired behaviour.
        let outputs = transfer_assets(
            &self.asset_storage_registry,
            &self.default_storage_name,
            &outputs,
        )?;
        Ok(Action {
            loc,
            kind: ActionKind::SetComplete { outputs },
        })
    }

    #[instrument(skip(self), err)]
    fn build_const_action(
        &self,
        loc: Location,
        value: serde_json::Value,
    ) -> miette::Result<Action> {
        let value_bytes = serde_json::to_vec(&value).into_diagnostic()?;
        let asset_key = save_asset(
            &self.asset_storage_registry,
            &self.default_storage_name,
            value_bytes,
        )?;

        let mut outputs = HashMap::new();
        outputs.insert("value".to_string(), asset_key);
        Ok(Action {
            loc,
            kind: ActionKind::SetComplete { outputs },
        })
    }

    #[instrument(skip(self, workflow_graph, node_states), err)]
    async fn build_output_actions(
        &self,
        workflow_graph: Arc<WorkflowGraph>,
        node_states: Arc<HashMap<NodeIndex, (NodeDefinition, NodeState)>>,
        n: NodeIndex,
        loc: Location,
    ) -> miette::Result<BoxStream<'_, miette::Result<Action>>> {
        let inputs = collect_inputs(&workflow_graph, &node_states, n)?;

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

    #[instrument(skip(self, workflow_graph, workflow_state, node_states), err)]
    async fn build_eval_actions(
        &self,
        workflow_graph: Arc<WorkflowGraph>,
        workflow_state: Arc<dyn WorkflowState>,
        node_states: Arc<HashMap<NodeIndex, (NodeDefinition, NodeState)>>,
        n: NodeIndex,
        loc: Location,
    ) -> miette::Result<LocalBoxStream<'_, miette::Result<Action>>> {
        // TODO: we don't need to collect inputs other than the graph
        // itself if the graph has already started.
        let inputs = collect_inputs(&workflow_graph, &node_states, n)?;

        let subgraph = if inputs.contains_key("graph") {
            self.load_subgraph(&inputs)?
        } else {
            workflow_graph
        };

        let subgraph_output_state = workflow_state
            .read(&loc.with_node(subgraph.output_idx()))
            .await?;

        if let Some(subgraph_outputs) = subgraph_output_state.outputs {
            // TODO: Unclear if this is desired behaviour.
            let outputs = transfer_assets(
                &self.asset_storage_registry,
                &self.default_storage_name,
                &subgraph_outputs,
            )?;
            return Ok(stream::once(future::ok(Action {
                loc,
                kind: ActionKind::SetComplete { outputs },
            }))
            .boxed());
        }

        let stream = self
            .build_actions(
                OrchestrationContext {
                    subworkflow_context: loc,
                    graph_inputs: inputs,
                    workflow_state: Arc::clone(&workflow_state),
                },
                subgraph,
            )
            .await?;

        Ok(stream.boxed_local())
    }

    #[instrument(skip(self, workflow_graph, workflow_state, node_states), err)]
    async fn build_loop_actions(
        &self,
        workflow_graph: Arc<WorkflowGraph>,
        workflow_state: Arc<dyn WorkflowState>,
        node_states: Arc<HashMap<NodeIndex, (NodeDefinition, NodeState)>>,
        n: NodeIndex,
        loc: Location,
        loop_index: Option<u32>,
    ) -> miette::Result<LocalBoxStream<'_, miette::Result<Action>>> {
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
                let mut inputs = collect_inputs(&workflow_graph, &node_states, n)?;
                let subgraph = self.load_subgraph(&inputs)?;

                let loop_loc = loc.with_loop_index(index);
                let loop_subgraph_output_loc = loop_loc.with_node(subgraph.output_idx());
                let loop_iteration_output_state =
                    workflow_state.read(&loop_subgraph_output_loc).await?;

                match loop_iteration_output_state.outputs {
                    None => {
                        if index > 0 {
                            let prev_loop_loc = loc.with_loop_index(index - 1);
                            let prev_loop_subgraph_output_loc =
                                prev_loop_loc.with_node(subgraph.output_idx());
                            let prev_loop_iteration_output_state =
                                workflow_state.read(&prev_loop_subgraph_output_loc).await?;

                            inputs.extend(prev_loop_iteration_output_state.outputs.ok_or_else(
                                || miette!("No outputs from previous loop iteration"),
                            )?);
                        }

                        return self
                            .build_actions(
                                OrchestrationContext {
                                    subworkflow_context: loop_loc,
                                    graph_inputs: inputs,
                                    workflow_state: Arc::clone(&workflow_state),
                                },
                                subgraph,
                            )
                            .await;
                    }
                    Some(outputs) => {
                        let should_continue_bytes =
                            load_asset(&self.asset_storage_registry, &outputs, "should_continue")?;

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

    #[instrument(skip(self, workflow_graph, workflow_state, node_states), err)]
    async fn build_map_actions(
        &self,
        workflow_graph: Arc<WorkflowGraph>,
        workflow_state: Arc<dyn WorkflowState>,
        node_states: Arc<HashMap<NodeIndex, (NodeDefinition, NodeState)>>,
        mapped_ports: HashSet<String>,
        n: NodeIndex,
        loc: Location,
        completed: Option<BitVec>,
    ) -> miette::Result<LocalBoxStream<'_, miette::Result<Action>>> {
        let inputs = collect_inputs(&workflow_graph, &node_states, n)?;
        let subgraph = self.load_subgraph(&inputs)?;

        Ok(match completed {
            None => {
                let mut input_sets = Vec::new();
                for mapped_port in mapped_ports {
                    let unfolded_assets =
                        unfold_asset(&self.asset_storage_registry, &inputs, &mapped_port)?;

                    if input_sets.is_empty() {
                        input_sets.extend(iter::repeat_n(inputs.clone(), unfolded_assets.len()));
                    }

                    for (index, asset) in unfolded_assets.into_iter().enumerate() {
                        let input_set = input_sets.get_mut(index).unwrap();
                        input_set.insert(mapped_port.clone(), asset);
                    }
                }
                let map_size = input_sets.len();

                let loc_copy = loc.clone();

                stream::iter(input_sets.into_iter().enumerate())
                    .flat_map_unordered(None, move |(index, inputs)| {
                        let map_loc = loc_copy.with_map_index(index as u32);
                        self.build_actions(
                            OrchestrationContext {
                                subworkflow_context: map_loc,
                                graph_inputs: inputs,
                                workflow_state: Arc::clone(&workflow_state),
                            },
                            subgraph.clone(),
                        )
                        .try_flatten_stream()
                        .boxed_local()
                    })
                    .chain(stream::once(future::ok(Action {
                        loc,
                        kind: ActionKind::SetRunningMap {
                            size: map_size as u32,
                        },
                    })))
                    .boxed_local()
            }
            Some(completed) => {
                let output_idx = subgraph.output_idx();

                if completed.all() {
                    return Ok(stream::once(async move {
                        let mut assets: HashMap<String, Vec<_>> = HashMap::new();
                        for index in 0..completed.len() {
                            let map_loc = loc.with_map_index(index as u32);
                            let subgraph_output_state =
                                workflow_state.read(&map_loc.with_node(output_idx)).await?;

                            let outputs = subgraph_output_state
                                .outputs
                                .ok_or_else(|| miette!("No outputs!"))?;

                            for (k, v) in outputs {
                                let entry = assets.entry(k).or_default();
                                entry.push(v);
                            }
                        }

                        let outputs = assets
                            .into_iter()
                            .map(|(k, v)| {
                                let asset_spec = fold_assets(
                                    &self.asset_storage_registry,
                                    &self.default_storage_name,
                                    v,
                                )?;

                                Ok((k, asset_spec))
                            })
                            .collect::<miette::Result<_>>()?;

                        Ok(Action {
                            loc,
                            kind: ActionKind::SetComplete { outputs },
                        })
                    })
                    .boxed());
                }

                stream::iter(completed.into_iter().enumerate())
                    .filter(|(_index, completed)| future::ready(!completed))
                    .flat_map_unordered(None, move |(index, _completed)| {
                        let loc_copy = loc.clone();
                        let map_loc = loc.with_map_index(index as u32);
                        let map_loc_copy = map_loc.clone();
                        self.build_actions(
                            OrchestrationContext {
                                subworkflow_context: map_loc,
                                graph_inputs: inputs.clone(),
                                workflow_state: Arc::clone(&workflow_state),
                            },
                            subgraph.clone(),
                        )
                        .try_flatten_stream()
                        .chain({
                            let workflow_state_copy = Arc::clone(&workflow_state);
                            async move {
                                let subgraph_output_state = workflow_state_copy
                                    .read(&map_loc_copy.with_node(output_idx))
                                    .await?;

                                match subgraph_output_state.outputs {
                                    None => Ok(stream::empty().boxed()),
                                    Some(_) => Ok(stream::once(future::ok(Action {
                                        loc: loc_copy,
                                        kind: ActionKind::SetMapElemComplete {
                                            index: index as u32,
                                        },
                                    }))
                                    .boxed()),
                                }
                            }
                            .try_flatten_stream()
                        })
                        .boxed_local()
                    })
                    .boxed_local()
            }
        })
    }

    #[instrument(skip(self, inputs), err)]
    fn load_subgraph(
        &self,
        inputs: &HashMap<String, AssetSpec>,
    ) -> miette::Result<Arc<WorkflowGraph>> {
        let subgraph_bytes = load_asset(&self.asset_storage_registry, inputs, "graph")?;
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
        mut actions: impl Stream<Item = miette::Result<Action>> + Unpin,
    ) -> miette::Result<()> {
        // Build a list of tasks to dispatch to Executors and immediately
        // process everything else.
        let mut task_plans = Vec::new();
        let mut event_sender = self.event_sender.clone();
        while let Some(Action { loc, kind }) = actions.next().await.transpose()? {
            match kind {
                ActionKind::PerformTask {
                    worker_name,
                    task_name,
                    inputs,
                } => task_plans.push(TaskPlan {
                    loc,
                    worker_name,
                    task_name,
                    inputs,
                    output_storage_name: Some(self.default_storage_name.clone()),
                    ..Default::default()
                }),
                ActionKind::SetSwitching { cond } => {
                    send_running_switching(&mut event_sender, loc, cond).await?;
                }
                ActionKind::SetRunningLoop { index } => {
                    send_running_loop(&mut event_sender, loc, index).await?;
                }
                ActionKind::SetRunningMap { size } => {
                    send_running_map(&mut event_sender, loc, size).await?;
                }
                ActionKind::SetMapElemComplete { index } => {
                    send_map_elem_complete(&mut event_sender, loc, index).await?;
                }
                ActionKind::SetComplete { outputs } => {
                    send_complete(&mut event_sender, loc, outputs).await?;
                }
                ActionKind::WorkflowFinished {} => {
                    send_workflow_run_complete(&mut event_sender).await?;
                }
            }
        }

        let default_executor_name = &self.default_executor_name;
        let executor = self
            .executor_registry
            .get(default_executor_name)
            .ok_or_else(|| miette!("Could not find a storage with name '{default_executor_name}' in ExecutorRegistry")).wrap_err("Could not run Task Nodes")?;
        executor
            .execute(task_plans)
            .await
            .wrap_err_with(|| miette!("Could not run Task Nodes"))?;

        Ok(())
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
    pub fn listen(&self) -> miette::Result<impl Stream<Item = Event> + use<>> {
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
            .collect::<miette::Result<Vec<BoxStream<Event>>>>()?;

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

#[instrument(skip(workflow_graph, node_states), err)]
fn build_task_action(
    workflow_graph: Arc<WorkflowGraph>,
    node_states: Arc<HashMap<NodeIndex, (NodeDefinition, NodeState)>>,
    n: NodeIndex,
    loc: Location,
    worker_name: &str,
    task_name: &str,
) -> miette::Result<Action> {
    let inputs = collect_inputs(&workflow_graph, &node_states, n)?;

    Ok(Action {
        loc,
        kind: ActionKind::PerformTask {
            worker_name: worker_name.to_string(),
            task_name: task_name.to_string(),
            inputs,
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

#[instrument(skip(workflow_graph, node_states))]
fn collect_inputs(
    workflow_graph: &WorkflowGraph,
    node_states: &HashMap<NodeIndex, (NodeDefinition, NodeState)>,
    n: NodeIndex,
) -> miette::Result<HashMap<String, AssetSpec>> {
    let mut inputs = HashMap::new();
    for (i, o) in workflow_graph.input_links(n) {
        let input_name = workflow_graph.get_port_name(i.into())?;
        let output_name = workflow_graph.get_port_name(o.into())?;
        let linked_node = workflow_graph.port_node(o)?;
        let (_, node_state) = node_states
            .get(&linked_node)
            .wrap_err_with(|| miette!("Could not find node outputs for node: {linked_node:?}"))?;
        let outputs = node_state
            .outputs
            .as_ref()
            .ok_or_else(|| miette!("Could not find node outputs for node: {linked_node:?}"))?;
        let output_asset_spec = outputs.get(output_name).ok_or_else(|| {
            miette!(
                "Could not get node output for node: {linked_node:?} and port name: {output_name}"
            )
        })?;

        inputs.insert(input_name.clone(), output_asset_spec.clone());
    }
    Ok(inputs)
}

#[cfg(test)]
mod tests {
    use futures::StreamExt;
    use rstest::rstest;
    use serde_json::json;

    use crate::{
        asset_storage::{assert_registry_contains_values, test_storage_registry},
        event::{NodeEvent, NodeStatus},
        executor::{
            inmemory::InMemoryExecutor, interface::Executor, subprocess::SubprocessExecutor,
        },
        graph::LegacyWorkflowGraph,
        state::{inmemory::InMemoryWorkflowState, interface::NodeState},
        updater::Updater,
    };

    use super::*;

    fn test_executor_registry(asset_storage_registry: &AssetStorageRegistry) -> ExecutorRegistry {
        let mut executor_registry: HashMap<String, Box<dyn Executor>> = HashMap::new();

        executor_registry.insert(
            "memory".to_string(),
            Box::new(InMemoryExecutor::try_new(asset_storage_registry, "memory").unwrap()),
        );
        executor_registry.insert(
            "subprocess".to_string(),
            Box::new(SubprocessExecutor::try_new(asset_storage_registry, "file", "file").unwrap()),
        );

        Arc::new(executor_registry)
    }

    // Test that we can plan a workflow with two input nodes.
    #[rstest]
    #[case("memory")]
    #[case("file")]
    #[tokio::test]
    async fn plan_two_input_workflow(#[case] default_storage_name: &str) -> miette::Result<()> {
        let (registry, input_sets, _dir) =
            test_storage_registry(vec![json!({"a": 1, "b": 4})], vec![]);
        let executor_registry = test_executor_registry(&registry);
        let orchestrator = Orchestrator::try_new(
            &registry,
            &executor_registry,
            default_storage_name,
            "memory",
        )?;

        let mut workflow_graph = WorkflowGraph::new(["out1".to_string(), "out2".to_string()]);
        let input1_idx = workflow_graph.add_node(
            NodeDefinition::Input {
                name: "a".to_string(),
            },
            [],
            ["a".to_string()],
        );
        workflow_graph
            .link_nodes_by_port_name(input1_idx, "a", workflow_graph.output_idx(), "out1")
            .unwrap();
        let input2_idx = workflow_graph.add_node(
            NodeDefinition::Input {
                name: "b".to_string(),
            },
            [],
            ["b".to_string()],
        );
        workflow_graph
            .link_nodes_by_port_name(input2_idx, "b", workflow_graph.output_idx(), "out2")
            .unwrap();
        let workflow_graph = Arc::new(workflow_graph);

        let (workflow_state, _state_events) = InMemoryWorkflowState::test();
        let workflow_state: Arc<dyn WorkflowState> = Arc::new(workflow_state);
        let inputs = input_sets[0].clone();
        let context = OrchestrationContext {
            subworkflow_context: Location::root(),
            graph_inputs: inputs.clone(),
            workflow_state,
        };
        let actions = orchestrator
            .build_actions(context, workflow_graph.clone())
            .await?;
        let actions = actions.collect::<Vec<_>>().await;

        assert_eq!(actions.len(), 2);
        let action0 = actions[0].as_ref().unwrap();
        assert!(matches!(action0.kind, ActionKind::SetComplete { .. }));
        let action1 = actions[1].as_ref().unwrap();
        assert!(matches!(action1.kind, ActionKind::SetComplete { .. }));

        Ok(())
    }

    // Test that we can plan the first actions of a workflow and run them.
    #[rstest]
    #[case("memory")]
    #[case("file")]
    #[tokio::test]
    async fn plan_and_run_simple_io_workflow(
        #[case] default_storage_name: &str,
    ) -> miette::Result<()> {
        let (registry, input_sets, _dir) = test_storage_registry(vec![json!({"a": 1})], vec![]);
        let executor_registry = test_executor_registry(&registry);
        let orchestrator = Orchestrator::try_new(
            &registry,
            &executor_registry,
            default_storage_name,
            "memory",
        )?;
        let mut stream = orchestrator.listen()?;

        let mut workflow_graph = WorkflowGraph::new(["out".to_string()]);
        let input_idx = workflow_graph.add_node(
            NodeDefinition::Input {
                name: "a".to_string(),
            },
            [],
            ["a".to_string()],
        );

        workflow_graph
            .link_nodes_by_port_name(input_idx, "a", workflow_graph.output_idx(), "out")
            .unwrap();
        let workflow_graph = Arc::new(workflow_graph);

        let inputs = input_sets[0].clone();
        let (workflow_state, _state_events) = InMemoryWorkflowState::test();
        let workflow_state: Arc<dyn WorkflowState> = Arc::new(workflow_state);
        let context = OrchestrationContext {
            subworkflow_context: Location::root(),
            graph_inputs: inputs.clone(),
            workflow_state: Arc::clone(&workflow_state),
        };
        let actions = orchestrator
            .build_actions(context, workflow_graph.clone())
            .await?;
        let actions = actions.collect::<Vec<_>>().await;

        assert_eq!(actions.len(), 1);
        let action0 = actions[0].as_ref().unwrap();
        assert_eq!(action0.loc, Location::from_node_index_iter([input_idx]));
        assert!(matches!(action0.kind, ActionKind::SetComplete { .. }));

        orchestrator.perform_actions(stream::iter(actions)).await?;
        let input_complete_event = stream.next().await.unwrap();
        let input_complete_outputs = input_complete_event.clone().outputs().unwrap();
        assert_registry_contains_values(
            &registry,
            &orchestrator.default_storage_name,
            &input_complete_outputs,
            json!({"a": 1}),
        );

        workflow_state.write(input_complete_event).await?;
        let context = OrchestrationContext {
            subworkflow_context: Location::root(),
            graph_inputs: inputs.clone(),
            workflow_state: Arc::clone(&workflow_state),
        };
        let actions = orchestrator
            .build_actions(context, workflow_graph.clone())
            .await?;
        let actions = actions.collect::<Vec<_>>().await;

        assert_eq!(actions.len(), 2);
        let action0 = actions[0].as_ref().unwrap();
        assert_eq!(
            action0.loc,
            Location::from_node_index_iter([workflow_graph.output_idx()])
        );
        assert!(matches!(action0.kind, ActionKind::SetComplete { .. }));
        let action1 = actions[1].as_ref().unwrap();
        assert_eq!(
            action1.loc,
            Location::from_node_index_iter([workflow_graph.output_idx()])
        );
        assert!(matches!(action1.kind, ActionKind::WorkflowFinished {}));

        orchestrator.perform_actions(stream::iter(actions)).await?;
        let output_complete_event = stream.next().await.unwrap();
        let output_complete_outputs = output_complete_event.outputs().unwrap();
        assert_registry_contains_values(
            &registry,
            &orchestrator.default_storage_name,
            &output_complete_outputs,
            json!({"out": 1}),
        );

        Ok(())
    }

    // Test that we can plan the first actions of a workflow and run them.
    #[rstest]
    #[case("memory")]
    #[case("file")]
    #[tokio::test]
    async fn plan_and_run_simple_eval_workflow(
        #[case] default_storage_name: &str,
    ) -> miette::Result<()> {
        let mut subworkflow_graph = WorkflowGraph::new(["out".to_string()]);
        let inner_a_input_idx = subworkflow_graph.add_node(
            NodeDefinition::Input {
                name: "a".to_string(),
            },
            [],
            ["a".to_string()],
        );

        subworkflow_graph
            .link_nodes_by_port_name(
                inner_a_input_idx,
                "a",
                subworkflow_graph.output_idx(),
                "out",
            )
            .unwrap();

        let (registry, input_sets, _dir) = test_storage_registry(
            vec![json!({"a": 1, "subworkflow": subworkflow_graph})],
            vec![],
        );
        let executor_registry = test_executor_registry(&registry);
        let orchestrator = Orchestrator::try_new(
            &registry,
            &executor_registry,
            default_storage_name,
            "memory",
        )?;
        let mut stream = orchestrator.listen()?;

        let mut workflow_graph = WorkflowGraph::new(["out".to_string()]);
        let a_input_idx = workflow_graph.add_node(
            NodeDefinition::Input {
                name: "a".to_string(),
            },
            [],
            ["a".to_string()],
        );
        let subworkflow_input_idx = workflow_graph.add_node(
            NodeDefinition::Input {
                name: "subworkflow".to_string(),
            },
            [],
            ["subworkflow".to_string()],
        );
        let eval_idx = workflow_graph.add_node(
            NodeDefinition::Eval {},
            ["graph".to_string(), "a".to_string()],
            ["out".to_string()],
        );

        workflow_graph
            .link_nodes_by_port_name(a_input_idx, "a", eval_idx, "a")
            .unwrap();
        workflow_graph
            .link_nodes_by_port_name(subworkflow_input_idx, "subworkflow", eval_idx, "graph")
            .unwrap();
        workflow_graph
            .link_nodes_by_port_name(eval_idx, "out", workflow_graph.output_idx(), "out")
            .unwrap();
        let workflow_graph = Arc::new(workflow_graph);

        let (workflow_state, _state_events) = InMemoryWorkflowState::test();
        let workflow_state: Arc<dyn WorkflowState> = Arc::new(workflow_state);
        let inputs = input_sets[0].clone();
        let context = OrchestrationContext {
            subworkflow_context: Location::root(),
            graph_inputs: inputs.clone(),
            workflow_state: Arc::clone(&workflow_state),
        };
        let actions = orchestrator
            .build_actions(context, workflow_graph.clone())
            .await?;
        let actions = actions.collect::<Vec<_>>().await;

        assert_eq!(actions.len(), 2);
        let action0 = actions[0].as_ref().unwrap();
        assert_eq!(
            action0.loc,
            Location::from_node_index_iter([subworkflow_input_idx])
        );
        assert!(matches!(action0.kind, ActionKind::SetComplete { .. }));
        let action1 = actions[1].as_ref().unwrap();
        assert_eq!(action1.loc, Location::from_node_index_iter([a_input_idx]));
        assert!(matches!(action1.kind, ActionKind::SetComplete { .. }));

        orchestrator.perform_actions(stream::iter(actions)).await?;
        let subworkflow_input_complete_event = stream.next().await.unwrap();
        let subworkflow_input_complete_outputs =
            subworkflow_input_complete_event.clone().outputs().unwrap();
        assert_registry_contains_values(
            &registry,
            &orchestrator.default_storage_name,
            &subworkflow_input_complete_outputs,
            json!({"subworkflow": subworkflow_graph}),
        );

        let a_input_complete_event = stream.next().await.unwrap();
        let a_input_complete_outputs = a_input_complete_event.clone().outputs().unwrap();
        assert_registry_contains_values(
            &registry,
            &orchestrator.default_storage_name,
            &a_input_complete_outputs,
            json!({"a": 1}),
        );

        workflow_state
            .write(subworkflow_input_complete_event)
            .await?;
        workflow_state.write(a_input_complete_event).await?;
        let context = OrchestrationContext {
            subworkflow_context: Location::root(),
            graph_inputs: inputs.clone(),
            workflow_state: Arc::clone(&workflow_state),
        };
        let actions = orchestrator
            .build_actions(context, workflow_graph.clone())
            .await?;
        let actions = actions.collect::<Vec<_>>().await;

        assert_eq!(actions.len(), 1);
        let action0 = actions[0].as_ref().unwrap();
        assert_eq!(
            action0.loc,
            Location::from_node_index_iter([eval_idx, inner_a_input_idx])
        );
        assert!(matches!(action0.kind, ActionKind::SetComplete { .. }));

        orchestrator.perform_actions(stream::iter(actions)).await?;
        let inner_a_input_complete_event = stream.next().await.unwrap();
        let inner_a_input_complete_outputs =
            inner_a_input_complete_event.clone().outputs().unwrap();
        assert_registry_contains_values(
            &registry,
            &orchestrator.default_storage_name,
            &inner_a_input_complete_outputs,
            json!({"a": 1}),
        );

        workflow_state.write(inner_a_input_complete_event).await?;
        let context = OrchestrationContext {
            subworkflow_context: Location::root(),
            graph_inputs: inputs.clone(),
            workflow_state: Arc::clone(&workflow_state),
        };
        let actions = orchestrator
            .build_actions(context, workflow_graph.clone())
            .await?;
        let actions = actions.collect::<Vec<_>>().await;

        orchestrator.perform_actions(stream::iter(actions)).await?;
        let inner_output_complete_event = stream.next().await.unwrap();
        let inner_output_complete_outputs = inner_output_complete_event.clone().outputs().unwrap();
        assert_registry_contains_values(
            &registry,
            &orchestrator.default_storage_name,
            &inner_output_complete_outputs,
            json!({"out": 1}),
        );

        let eval_complete_event = Event::Node(NodeEvent {
            loc: Location::from_node_index_iter([eval_idx]),
            status: NodeStatus::Complete {
                outputs: inner_output_complete_outputs.clone(),
            },
        });
        workflow_state.write(inner_output_complete_event).await?;
        workflow_state.write(eval_complete_event).await?;
        let context = OrchestrationContext {
            subworkflow_context: Location::root(),
            graph_inputs: inputs.clone(),
            workflow_state: Arc::clone(&workflow_state),
        };
        let actions = orchestrator
            .build_actions(context, workflow_graph.clone())
            .await?;
        let actions = actions.collect::<Vec<_>>().await;

        orchestrator.perform_actions(stream::iter(actions)).await?;
        let output_complete_event = stream.next().await.unwrap();
        let output_complete_outputs = output_complete_event.outputs().unwrap();
        assert_registry_contains_values(
            &registry,
            &orchestrator.default_storage_name,
            &output_complete_outputs,
            json!({"out": 1}),
        );

        Ok(())
    }

    // Test that we can plan the first actions of a workflow and run them.
    #[rstest]
    #[case("memory")]
    #[case("file")]
    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    async fn run_simple_task_workflow(#[case] default_storage_name: &str) -> miette::Result<()> {
        let mut workflow_graph = WorkflowGraph::new(["out".to_string()]);
        let input_idx = workflow_graph.add_node(
            NodeDefinition::Input {
                name: "a".to_string(),
            },
            [],
            ["a".to_string()],
        );
        let const_idx = workflow_graph.add_node(
            NodeDefinition::Const { value: 3.into() },
            [],
            ["value".to_string()],
        );
        let add_idx = workflow_graph.add_node(
            NodeDefinition::Task {
                worker_name: "builtin".to_string(),
                task_name: "iadd".to_string(),
            },
            ["a".to_string(), "b".to_string()],
            ["value".to_string()],
        );

        let output_idx = workflow_graph.output_idx();
        workflow_graph
            .link_nodes_by_port_name(input_idx, "a", add_idx, "a")
            .unwrap();
        workflow_graph
            .link_nodes_by_port_name(const_idx, "value", add_idx, "b")
            .unwrap();
        workflow_graph
            .link_nodes_by_port_name(add_idx, "value", output_idx, "out")
            .unwrap();
        let workflow_graph = Arc::new(workflow_graph);

        let (registry, input_sets, _dir) = test_storage_registry(vec![json!({"a": 1})], vec![]);
        let executor_registry = test_executor_registry(&registry);
        let orchestrator = Orchestrator::try_new(
            &registry,
            &executor_registry,
            default_storage_name,
            "memory",
        )?;

        let (workflow_state, state_events) = InMemoryWorkflowState::test();
        let workflow_state: Arc<dyn WorkflowState> = Arc::new(workflow_state);
        let inputs = input_sets[0].clone();
        let context = OrchestrationContext {
            subworkflow_context: Location::root(),
            graph_inputs: inputs.clone(),
            workflow_state: Arc::clone(&workflow_state),
        };
        let updater = Updater::new(Arc::clone(&workflow_state));
        let stream = orchestrator.listen()?;
        let _task = tokio::spawn(async move {
            updater.process(stream).await.unwrap();
            println!("done listening");
        });
        let actions = orchestrator
            .build_actions(context.clone(), workflow_graph.clone())
            .await?;
        orchestrator.perform_actions(actions).await?;

        let mut state_chunks = state_events.ready_chunks(8);
        while let Some(chunk) = state_chunks.next().await {
            if chunk.iter().any(|updated| updated.stopped) {
                break;
            }
            let actions = orchestrator
                .build_actions(context.clone(), workflow_graph.clone())
                .await?;
            orchestrator.perform_actions(actions).await?;
        }

        let a_input_state = workflow_state
            .read(&Location::from_node_index_iter([input_idx]))
            .await?;

        assert!(matches!(
            a_input_state,
            NodeState {
                complete_time: Some(..),
                outputs: Some(..),
                ..
            }
        ));

        let subworkflow_input_state = workflow_state
            .read(&Location::from_node_index_iter([const_idx]))
            .await?;

        assert!(matches!(
            subworkflow_input_state,
            NodeState {
                complete_time: Some(..),
                outputs: Some(..),
                ..
            }
        ));

        let output_state = workflow_state
            .read(&Location::from_node_index_iter([output_idx]))
            .await?;

        let outputs = output_state.outputs.expect("no outputs found");

        assert_registry_contains_values(
            &registry,
            default_storage_name,
            &outputs,
            json!({"out": 4}),
        );

        Ok(())
    }

    // Test that we can plan the first actions of a workflow and run them.
    #[rstest]
    #[case("memory")]
    #[case("file")]
    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    async fn run_simple_eval_workflow(#[case] default_storage_name: &str) -> miette::Result<()> {
        let mut subworkflow_graph = WorkflowGraph::new(["out".to_string()]);
        let inner_a_input_idx = subworkflow_graph.add_node(
            NodeDefinition::Input {
                name: "a".to_string(),
            },
            [],
            ["a".to_string()],
        );

        subworkflow_graph
            .link_nodes_by_port_name(
                inner_a_input_idx,
                "a",
                subworkflow_graph.output_idx(),
                "out",
            )
            .unwrap();

        let mut workflow_graph = WorkflowGraph::new(["out".to_string()]);
        let a_input_idx = workflow_graph.add_node(
            NodeDefinition::Input {
                name: "a".to_string(),
            },
            [],
            ["a".to_string()],
        );
        let subworkflow_input_idx = workflow_graph.add_node(
            NodeDefinition::Input {
                name: "subworkflow".to_string(),
            },
            [],
            ["subworkflow".to_string()],
        );
        let eval_idx = workflow_graph.add_node(
            NodeDefinition::Eval {},
            ["graph".to_string(), "a".to_string()],
            ["out".to_string()],
        );

        workflow_graph
            .link_nodes_by_port_name(a_input_idx, "a", eval_idx, "a")
            .unwrap();
        workflow_graph
            .link_nodes_by_port_name(subworkflow_input_idx, "subworkflow", eval_idx, "graph")
            .unwrap();
        workflow_graph
            .link_nodes_by_port_name(eval_idx, "out", workflow_graph.output_idx(), "out")
            .unwrap();
        let workflow_graph = Arc::new(workflow_graph);

        let (registry, input_sets, _dir) = test_storage_registry(
            vec![json!({"a": 1, "subworkflow": subworkflow_graph})],
            vec![],
        );
        let executor_registry = test_executor_registry(&registry);
        let orchestrator = Orchestrator::try_new(
            &registry,
            &executor_registry,
            default_storage_name,
            "memory",
        )?;

        let (workflow_state, state_events) = InMemoryWorkflowState::test();
        let workflow_state: Arc<dyn WorkflowState> = Arc::new(workflow_state);
        let inputs = input_sets[0].clone();
        let context = OrchestrationContext {
            subworkflow_context: Location::root(),
            graph_inputs: inputs.clone(),
            workflow_state: Arc::clone(&workflow_state),
        };
        let updater = Updater::new(Arc::clone(&workflow_state));
        let stream = orchestrator.listen()?;
        let _task = tokio::spawn(async move {
            updater.process(stream).await.unwrap();
            println!("done listening");
        });
        let actions = orchestrator
            .build_actions(context.clone(), workflow_graph.clone())
            .await?;
        orchestrator.perform_actions(actions).await?;
        let mut state_chunks = state_events.ready_chunks(8);
        while let Some(chunk) = state_chunks.next().await {
            if chunk.iter().any(|updated| updated.stopped) {
                break;
            }
            let actions = orchestrator
                .build_actions(context.clone(), workflow_graph.clone())
                .await?;
            orchestrator.perform_actions(actions).await?;
        }

        let a_input_state = workflow_state
            .read(&Location::from_node_index_iter([a_input_idx]))
            .await?;

        assert!(matches!(
            a_input_state,
            NodeState {
                complete_time: Some(..),
                outputs: Some(..),
                ..
            }
        ));

        let subworkflow_input_state = workflow_state
            .read(&Location::from_node_index_iter([subworkflow_input_idx]))
            .await?;

        assert!(matches!(
            subworkflow_input_state,
            NodeState {
                complete_time: Some(..),
                outputs: Some(..),
                ..
            }
        ));

        let output_state = workflow_state
            .read(&Location::from_node_index_iter([
                workflow_graph.output_idx()
            ]))
            .await?;

        let outputs = output_state.outputs.expect("no outputs found");

        assert_registry_contains_values(
            &registry,
            &orchestrator.default_storage_name,
            &outputs,
            json!({"out": 1}),
        );

        Ok(())
    }

    // Test that we can plan the first actions of a workflow and run them.
    #[rstest]
    #[case("memory")]
    #[case("file")]
    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    async fn run_serialized_workflow(#[case] default_storage_name: &str) -> miette::Result<()> {
        let serialized_graph = include_str!("../tests/cli/data/sample_graph");
        let graph: LegacyWorkflowGraph =
            serde_json::from_str(serialized_graph).into_diagnostic()?;
        let workflow_graph = Arc::new(graph.to_workflow_graph().unwrap());

        let (registry, _input_sets, _dir) = test_storage_registry(vec![], vec![]);
        let executor_registry = test_executor_registry(&registry);
        let orchestrator = Orchestrator::try_new(
            &registry,
            &executor_registry,
            default_storage_name,
            "memory",
        )?;

        let (workflow_state, state_events) = InMemoryWorkflowState::test();
        let workflow_state: Arc<dyn WorkflowState> = Arc::new(workflow_state);
        let context = OrchestrationContext {
            subworkflow_context: Location::root(),
            graph_inputs: HashMap::new(),
            workflow_state: Arc::clone(&workflow_state),
        };
        let updater = Updater::new(Arc::clone(&workflow_state));
        let stream = orchestrator.listen()?;
        let _task = tokio::spawn(async move {
            updater.process(stream).await.unwrap();
            println!("done listening");
        });
        let actions = orchestrator
            .build_actions(context.clone(), workflow_graph.clone())
            .await?;
        orchestrator.perform_actions(actions).await?;
        let mut state_chunks = state_events.ready_chunks(8);
        while let Some(chunk) = state_chunks.next().await {
            if chunk.iter().any(|updated| updated.stopped) {
                break;
            }
            let actions = orchestrator
                .build_actions(context.clone(), workflow_graph.clone())
                .await?;
            orchestrator.perform_actions(actions).await?;
        }

        let output_state = workflow_state
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
        );

        Ok(())
    }
}
