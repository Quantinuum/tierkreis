/*!
This module defines the interface contracts that the various [`RuntimeState`]
and [`WorkflowRunState`] implementations must satisfy.
*/
use std::{
    collections::{HashMap, HashSet},
    fmt::Debug,
    sync::Arc,
};

use bitvec::vec::BitVec;
use chrono::{DateTime, Utc};
use futures::{FutureExt, future::BoxFuture};
use miette::miette;
use tokio::sync::watch;
use uuid::Uuid;

use crate::{
    asset_storage::{AssetSpec, AssetStorageRegistry},
    event::WorkflowRunEvent,
    executor::interface::TaskHandle,
    graph::{NodeDefinition, WorkflowGraph},
    location::Location,
    workflow_state_query::{dependents, resolve_location},
};

/// [`RuntimeWatchState`] is a struct that is updated by the [`RuntimeState`] interface
/// whenever a run attempt changes, in order to drive further workflow orchestration.
///
/// Not necessarily representative of all workflow runs that are not finished,
/// but rather an in-memory cache of what is used by the runtime to decide
/// what to run next.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct RuntimeWatchState {
    /// The set of runs currently being executed by the runtime.
    pub active_runs: HashSet<(Uuid, u32)>,
}

/// [`WorkflowRunStateSummary`] is a struct that stores the overall state of the
/// workflow as a summary.
#[derive(Debug, Clone)]
pub struct WorkflowRunStateSummary {
    /// The run identifier.
    pub run_id: uuid::Uuid,
    /// The attempt number.
    pub attempt: u32,
    /// The workflow graph identifier.
    pub workflow_id: uuid::Uuid,
    /// The name of the workflow.
    pub name: Option<String>,
    /// The time that the workflow started.
    pub started_time: Option<chrono::DateTime<Utc>>,
    /// The time that the workflow was queued.
    pub queued_time: Option<chrono::DateTime<Utc>>,
    /// The time that the workflow completed.
    pub complete_time: Option<chrono::DateTime<Utc>>,
    /// The time that the workflow was cancelled.
    pub cancelled_time: Option<chrono::DateTime<Utc>>,
    /// The time that the workflow errored.
    pub error_time: Option<chrono::DateTime<Utc>>,
    /// Locations of nodes that have errored in this run.
    pub errored_locations: Vec<Location>,
}

/// [`NodeState`] is a struct that stores the possible state that a node
/// in the Workflow graph can be in.
///
/// This state is built up by reading [`Event`] messages and can be queried
/// by the [`WorkflowRunState`] interface.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct NodeState {
    /// The user specified name for the Node if any.
    pub name: Option<String>,
    /// The time at which the node was scheduled by the [`Orchestrator`] if any.
    pub scheduled_time: Option<DateTime<Utc>>,
    /// The time at which the node was queued by an [`Executor`] if any.
    pub queued_time: Option<DateTime<Utc>>,
    /// The time at which the node started running by an [`Executor`] if any.
    pub running_time: Option<DateTime<Utc>>,
    /// The time at which the node was completed by the [`Orchestrator`] or an [`Executor`] if any.
    pub complete_time: Option<DateTime<Utc>>,
    /// The time at which the node was cancelled by the [`Executor`] if any.
    pub cancelled_time: Option<DateTime<Utc>>,
    /// The time at which the node errored as reported by the [`Orchestrator`] or an [`Executor`] if any.
    pub error_time: Option<DateTime<Utc>>,

    /// The outputs of the node and their stored locations if any.
    pub outputs: Option<HashMap<String, AssetSpec>>,

    /// The state associated with the `pred` port if this Node is an `IfElse` node.
    pub cond: Option<bool>,
    /// The state associated with the loop index if this Node is a `Loop` node.
    pub loop_index: Option<u32>,
    /// The state associated with a map if this Node is a `Map` node.
    ///
    /// This value tracks the number of elements being mapped over.
    pub map_completed: Option<BitVec<u8>>,

    /// The error message of the node if any.
    pub error: Option<String>,
    /// The detail of the error for the node if any.
    pub error_detail: Option<String>,

    /// The handle to the node
    pub handle: Option<TaskHandle>,
}

/// Describes the state which must not be copied into a restarted run attempt.
#[derive(Debug, Clone)]
pub struct RestartPlan {
    invalidated: HashSet<Location>,
    truncated: HashSet<Location>,
    loop_restarts: HashMap<Location, u32>,
}

impl RestartPlan {
    pub(crate) fn new(invalidated: HashSet<Location>, truncated: HashSet<Location>) -> Self {
        let loop_restarts = truncated
            .iter()
            .filter_map(|container| {
                invalidated
                    .iter()
                    .filter_map(|location| {
                        if !container.is_ancestor(location) {
                            return None;
                        }
                        location
                            .components()
                            .nth(container.components().count())
                            .and_then(|component| match component {
                                crate::location::LocationComponent::LoopIndex { index } => {
                                    Some(*index)
                                }
                                _ => None,
                            })
                    })
                    .min()
                    .map(|index| (container.clone(), index))
            })
            .collect();

        Self {
            invalidated,
            truncated,
            loop_restarts,
        }
    }

    /// Whether a location is fully invalidated, including locations below an
    /// invalidated iteration or container.
    #[must_use]
    pub fn excludes(&self, location: &Location) -> bool {
        self.invalidated
            .iter()
            .any(|invalidated| invalidated.contains(location))
    }

    /// Whether the state at exactly this container location is truncated.
    #[must_use]
    pub fn truncates(&self, location: &Location) -> bool {
        self.truncated.contains(location)
    }

    /// The iteration from which a truncated loop must resume, if applicable.
    #[must_use]
    pub fn loop_restart(&self, location: &Location) -> Option<u32> {
        self.loop_restarts.get(location).copied()
    }

    fn into_invalidated(self) -> HashSet<Location> {
        self.invalidated
    }
}

/// [`RuntimeState`] is an interface to the state of the overall tierkreis runtime, across
/// all of the running and completed Workflows.
///
/// At creation time  (or after a potential crash) implementations should re-populate the
/// in-memory [`RuntimeWatchState`] from any runs that were not in a terminal
/// state when the process last exited. Implementations without durable storage
/// may treat this as a no-op.
pub trait RuntimeState: Debug + Send + Sync {
    /// Retrieve the [`WorkflowGraph`] specified by id.
    fn load_workflow(
        &self,
        workflow_id: Uuid,
    ) -> BoxFuture<'_, miette::Result<(Option<String>, WorkflowGraph)>>;
    /// Save a [`WorkflowGraph`] and return a new id.
    fn save_workflow(
        &self,
        name: Option<String>,
        workflow_graph: WorkflowGraph,
    ) -> BoxFuture<'_, miette::Result<Uuid>>;

    /// Create a new [`WorkflowRunState`] for a Workflow in the [`RuntimeState`] specified by id.
    ///
    /// The new workflow run should be included in the active runs of `RuntimeWatchState`.
    fn new_workflow_run_state(
        &self,
        workflow_id: Uuid,
        inputs: HashMap<String, AssetSpec>,
    ) -> BoxFuture<'_, miette::Result<Arc<dyn WorkflowRunState>>>;
    /// Retrieve a handle to a [`WorkflowRunState`] depending on the `run_id` and attempt number.
    ///
    /// If the backing data for the [`WorkflowRunState`] does not exist, create it.
    fn load_workflow_run_state(
        &self,
        run_id: Uuid,
        attempt: u32,
    ) -> BoxFuture<'_, miette::Result<Arc<dyn WorkflowRunState>>>;
    /// Listen for updates about *all* of the running workflows.
    fn listen(&self) -> watch::Receiver<RuntimeWatchState>;

    /// List summaries of all workflow runs in the runtime state.
    fn list_workflow_run_summaries(
        &self,
    ) -> BoxFuture<'_, miette::Result<Vec<WorkflowRunStateSummary>>>;

    /// Create a new attempt for an existing `run_id`
    ///
    /// ### Errors
    ///
    /// Should error when there already is an active run.
    ///
    /// attempt(new) = max(attempts) + 1, should always be sequential
    /// This can be a partial restart, e.g., if some nodes errored.
    fn new_attempt(&self, run_id: Uuid)
    -> BoxFuture<'_, miette::Result<Arc<dyn WorkflowRunState>>>;

    /// Atomically create a new attempt by copying reusable state and inputs
    /// from `source` according to `restart_plan`.
    fn new_attempt_from<'a>(
        &'a self,
        source: &'a dyn WorkflowRunState,
        restart_plan: &'a RestartPlan,
    ) -> BoxFuture<'a, miette::Result<Arc<dyn WorkflowRunState>>>;

    /// Restart Task nodes in a new attempt, preserving unaffected node state
    /// and the persisted inputs required to resume invalidated subgraphs.
    ///
    /// An empty `locs` restarts all errored nodes from the source attempt.
    /// Returns the new attempt number and sorted fully invalidated locations.
    ///
    /// # Errors
    ///
    /// Returns Err if there are no errored nodes, a location is not a Task,
    /// graph resolution fails, or the new attempt cannot be created or populated.
    fn restart_task<'a>(
        &'a self,
        asset_storage_registry: &'a AssetStorageRegistry,
        run_id: Uuid,
        attempt: u32,
        locs: Vec<Location>,
    ) -> BoxFuture<'a, miette::Result<(u32, Vec<Location>)>> {
        async move {
            let source_state = self.load_workflow_run_state(run_id, attempt).await?;
            let locs = if locs.is_empty() {
                let errored_locations = source_state.summary().await?.errored_locations;
                if errored_locations.is_empty() {
                    return Err(miette!("No errored nodes to restart"));
                }
                errored_locations
            } else {
                locs
            };
            let workflow_id = source_state.workflow_id();
            let (_workflow_name, workflow_graph) = self.load_workflow(workflow_id).await?;
            let workflow_graph = Arc::new(workflow_graph);

            let restart_plan = compute_restart_plan(
                asset_storage_registry,
                &source_state,
                &workflow_graph,
                locs.iter(),
            )
            .await?;

            let new_state = self.new_attempt_from(&*source_state, &restart_plan).await?;

            let mut invalidated: Vec<Location> =
                restart_plan.into_invalidated().into_iter().collect();
            invalidated.sort_by_key(ToString::to_string);

            Ok((new_state.attempt(), invalidated))
        }
        .boxed()
    }
}

/// [`WorkflowRunState`] is an interface to the state of an individual Workflow run attempt.
pub trait WorkflowRunState: Debug + Send + Sync {
    /// Retrieve the id for the [`WorkflowGraph`] associated with this Workflow run attempt.
    fn workflow_id(&self) -> Uuid;
    /// Retrieve the `run_id` associated with this `WorkflowRunState`.
    fn run_id(&self) -> Uuid;
    /// Retrieve the `attempt` associated with this `WorkflowRunState`.
    fn attempt(&self) -> u32;
    /// Retrieve the `WorkflowRunStateSummary` at the current point in time for this `WorkflowRunState`.
    fn summary(&self) -> BoxFuture<'_, miette::Result<WorkflowRunStateSummary>>;
    /// Retrieve the workflow inputs associated with this `WorkflowRunState`.
    fn load_inputs(&self) -> BoxFuture<'_, miette::Result<HashMap<String, AssetSpec>>>;
    /// Retrieve the workflow inputs associated with this `WorkflowRunState`.
    fn load_named_input_from_location<'a>(
        &'a self,
        location: &'a Location,
        name: &'a str,
    ) -> BoxFuture<'a, miette::Result<AssetSpec>>;
    /// Update the [`WorkflowRunState`] from a [`WorkflowRunEvent`].
    fn write(&self, event: WorkflowRunEvent) -> BoxFuture<'_, miette::Result<()>>;
    /// Read the state of a Node at the specified [`Location`].
    ///
    /// If the `location` has no existing state, a default [`NodeState`] will be returned.
    fn read<'a>(&'a self, location: &'a Location) -> BoxFuture<'a, miette::Result<NodeState>>;
    /// Read the state of potentially many nodes with the specified name.
    fn read_by_name<'a>(
        &'a self,
        name: &'a str,
    ) -> BoxFuture<'a, miette::Result<HashMap<Location, NodeState>>>;
    /// Read the state of many Nodes at the specified [`Location`]s.
    ///
    /// If the `location` has no existing state, no entry will be returned for that [`Location`].
    fn read_many<'a>(
        &'a self,
        locations: &'a mut (dyn Iterator<Item = Location> + Send),
    ) -> BoxFuture<'a, miette::Result<HashMap<Location, NodeState>>>;
    /// Add metadata for the Workflow run. The new metadata will be merged with the existing values.
    fn add_metadata(&self, metadata: HashMap<String, String>) -> BoxFuture<'_, miette::Result<()>>;
    /// Read the metadata for the Workflow run.
    fn read_metadata(&self) -> BoxFuture<'_, miette::Result<HashMap<String, String>>>;
}

/// Return a set of [`Location`] to exclude from copying and a set to truncate
/// based the invalidation of the provided locations and existing workflow state.
async fn compute_restart_plan(
    asset_storage_registry: &AssetStorageRegistry,
    source_state: &Arc<dyn WorkflowRunState>,
    workflow_graph: &Arc<WorkflowGraph>,
    locs: impl Iterator<Item = &Location>,
) -> miette::Result<RestartPlan> {
    let mut exclude = HashSet::new();
    let mut truncate = HashSet::new();

    for loc in locs {
        let (graph, _, node) =
            resolve_location(asset_storage_registry, source_state, workflow_graph, loc).await?;
        if !matches!(
            graph.node_definition(node),
            Some(NodeDefinition::Task { .. })
        ) {
            return Err(miette!("Can only restart Task nodes, {loc} is not a Task"));
        }

        exclude.insert(loc.clone());
        exclude
            .extend(dependents(asset_storage_registry, source_state, workflow_graph, loc).await?);
        truncate.extend(loc.node_ancestors());
    }
    truncate.retain(|loc| !exclude.contains(loc));

    Ok(RestartPlan::new(exclude, truncate))
}

#[cfg(test)]
async fn compute_exclude_and_truncate(
    asset_storage_registry: &AssetStorageRegistry,
    source_state: &Arc<dyn WorkflowRunState>,
    workflow_graph: &Arc<WorkflowGraph>,
    locs: impl Iterator<Item = &Location>,
) -> miette::Result<(HashSet<Location>, HashSet<Location>)> {
    let plan =
        compute_restart_plan(asset_storage_registry, source_state, workflow_graph, locs).await?;
    Ok((plan.invalidated, plan.truncated))
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use crate::{
        asset_storage::{AssetKey, AssetKind, AssetSpec, test_storage_registry},
        builder::{constant, eval, link, loop_node, map_node, task},
        event::{NodeEvent, NodeStatus, RunningStateUpdate},
        state::{InMemoryRuntimeState, SqliteRuntimeState},
    };

    use super::*;

    fn example_task_graph() -> miette::Result<WorkflowGraph> {
        let mut graph = WorkflowGraph::new(["out".to_string()]);
        let output_node = graph.output_idx();
        let task_node = task(&mut graph, "foo", "bar", [], ["out"]);
        link(&mut graph, (task_node, "out"), (output_node, "out"))?;

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

    fn test_asset() -> AssetSpec {
        AssetSpec {
            kind: AssetKind::Memory,
            storage_name: "test".to_string(),
            asset_key: AssetKey::new(),
        }
    }

    async fn restart_copy_contract(runtime: &dyn RuntimeState) -> miette::Result<()> {
        let root_asset = test_asset();
        let workflow_id = runtime.save_workflow(None, example_loop_graph()?).await?;
        let source = runtime
            .new_workflow_run_state(
                workflow_id,
                HashMap::from([("root".to_string(), root_asset.clone())]),
            )
            .await?;
        let loop_loc = Location::new("N2")?;
        let selected_iteration = Location::new("N2.L1")?;
        let later_iteration = Location::new("N2.L2")?;
        let selected_task = Location::new("N2.L1.N1")?;
        let later_task = Location::new("N2.L2.N1")?;
        let unaffected_task = Location::new("N9")?;
        let selected_input = test_asset();
        let later_input = test_asset();
        let output = HashMap::from([("value".to_string(), test_asset())]);

        for (index, input) in [
            (0, test_asset()),
            (1, selected_input.clone()),
            (2, later_input),
        ] {
            source
                .write(WorkflowRunEvent::NodeEvents(vec![NodeEvent {
                    loc: loop_loc.clone(),
                    status: NodeStatus::Running {
                        state_update: Some(RunningStateUpdate::Looping {
                            index,
                            inputs: HashMap::from([("value".to_string(), input)]),
                        }),
                    },
                    name: None,
                }]))
                .await?;
        }
        source
            .write(WorkflowRunEvent::NodeEvents(vec![
                NodeEvent {
                    loc: selected_task.clone(),
                    status: NodeStatus::Complete {
                        outputs: output.clone(),
                    },
                    name: None,
                },
                NodeEvent {
                    loc: later_task.clone(),
                    status: NodeStatus::Complete {
                        outputs: output.clone(),
                    },
                    name: None,
                },
                NodeEvent {
                    loc: unaffected_task.clone(),
                    status: NodeStatus::Complete { outputs: output },
                    name: None,
                },
            ]))
            .await?;
        source.write(WorkflowRunEvent::Completed {}).await?;

        let plan = RestartPlan::new(
            HashSet::from([selected_task.clone(), later_iteration.clone()]),
            HashSet::from([loop_loc.clone()]),
        );
        let restarted = runtime.new_attempt_from(&*source, &plan).await?;

        assert_eq!(restarted.attempt(), 1);
        assert_eq!(restarted.load_inputs().await?["root"], root_asset);
        assert_eq!(
            restarted
                .load_named_input_from_location(&selected_iteration, "value")
                .await?,
            selected_input
        );
        assert!(
            restarted
                .load_named_input_from_location(&later_iteration, "value")
                .await
                .is_err()
        );
        assert!(restarted.read(&selected_task).await?.outputs.is_none());
        assert!(restarted.read(&later_task).await?.outputs.is_none());
        assert!(restarted.read(&unaffected_task).await?.outputs.is_some());

        let loop_state = restarted.read(&loop_loc).await?;
        assert_eq!(loop_state.loop_index, Some(1));
        assert!(loop_state.running_time.is_none());
        assert!(loop_state.outputs.is_none());

        // Copying into the new attempt must never mutate the source attempt.
        assert!(source.read(&selected_task).await?.outputs.is_some());
        assert_eq!(source.read(&loop_loc).await?.loop_index, Some(2));

        Ok(())
    }

    #[tokio::test]
    async fn restart_copy_contract_in_memory() -> miette::Result<()> {
        restart_copy_contract(&InMemoryRuntimeState::new()).await
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn restart_copy_contract_sqlite() -> miette::Result<()> {
        restart_copy_contract(&SqliteRuntimeState::try_new_in_memory().await?).await
    }

    #[tokio::test]
    async fn exclude_and_truncate_task() -> miette::Result<()> {
        let (asset_storage_registry, _, _dir) = test_storage_registry([], []).await;
        let state = Arc::new(InMemoryRuntimeState::new());
        let source_state = state
            .new_workflow_run_state(Uuid::now_v7(), HashMap::new())
            .await?;

        let graph = Arc::new(example_task_graph()?);

        let locs = &[Location::new("N1")?];

        let (exclude, truncate) = compute_exclude_and_truncate(
            &asset_storage_registry,
            &source_state,
            &graph,
            locs.iter(),
        )
        .await?;

        assert_eq!(exclude.len(), 2);
        assert!(exclude.contains(&Location::new("N1")?));
        assert!(exclude.contains(&Location::new("N0")?));
        assert!(truncate.is_empty());

        Ok(())
    }

    #[tokio::test]
    async fn exclude_and_truncate_task_in_eval() -> miette::Result<()> {
        let (asset_storage_registry, asset_sets, _dir) =
            test_storage_registry([json!({"value": example_task_graph()?})], []).await;
        let state = Arc::new(InMemoryRuntimeState::new());
        let source_state = state
            .new_workflow_run_state(Uuid::now_v7(), HashMap::new())
            .await?;

        let graph = Arc::new(example_eval_graph()?);

        let locs = &[Location::new("N2.N1")?];

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

        let (exclude, truncate) = compute_exclude_and_truncate(
            &asset_storage_registry,
            &source_state,
            &graph,
            locs.iter(),
        )
        .await?;

        assert_eq!(exclude.len(), 3);
        assert!(exclude.contains(&Location::new("N2.N1")?));
        assert!(exclude.contains(&Location::new("N2.N0")?));
        assert!(exclude.contains(&Location::new("N0")?));
        assert_eq!(truncate.len(), 1);
        assert!(truncate.contains(&Location::new("N2")?));

        Ok(())
    }

    #[tokio::test]
    async fn exclude_and_truncate_task_in_loop() -> miette::Result<()> {
        let (asset_storage_registry, asset_sets, _dir) =
            test_storage_registry([json!({"value": example_task_graph()?})], []).await;
        let state = Arc::new(InMemoryRuntimeState::new());
        let source_state = state
            .new_workflow_run_state(Uuid::now_v7(), HashMap::new())
            .await?;

        let graph = Arc::new(example_loop_graph()?);
        let locs = &[Location::new("N2.L0.N1")?];

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

        let (exclude, truncate) = compute_exclude_and_truncate(
            &asset_storage_registry,
            &source_state,
            &graph,
            locs.iter(),
        )
        .await?;

        assert_eq!(exclude.len(), 3);
        assert!(exclude.contains(&Location::new("N2.L0.N1")?));
        assert!(exclude.contains(&Location::new("N2.L0.N0")?));
        assert!(exclude.contains(&Location::new("N0")?));
        assert_eq!(truncate.len(), 1);
        assert!(truncate.contains(&Location::new("N2")?));

        Ok(())
    }

    #[tokio::test]
    async fn exclude_and_truncate_task_in_loop_iteration() -> miette::Result<()> {
        let (asset_storage_registry, asset_sets, _dir) =
            test_storage_registry([json!({"value": example_task_graph()?})], []).await;
        let state = Arc::new(InMemoryRuntimeState::new());
        let source_state = state
            .new_workflow_run_state(Uuid::now_v7(), HashMap::new())
            .await?;

        let graph = Arc::new(example_loop_graph()?);
        let locs = &[Location::new("N2.L1.N1")?];

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

        let (exclude, truncate) = compute_exclude_and_truncate(
            &asset_storage_registry,
            &source_state,
            &graph,
            locs.iter(),
        )
        .await?;

        assert_eq!(exclude.len(), 3);
        assert!(exclude.contains(&Location::new("N2.L1.N1")?));
        assert!(exclude.contains(&Location::new("N2.L1.N0")?));
        assert!(exclude.contains(&Location::new("N0")?));
        assert_eq!(truncate.len(), 1);
        assert!(truncate.contains(&Location::new("N2")?));

        Ok(())
    }

    #[tokio::test]
    async fn exclude_and_truncate_task_in_loop_mid_iteration() -> miette::Result<()> {
        let (asset_storage_registry, asset_sets, _dir) =
            test_storage_registry([json!({"value": example_task_graph()?})], []).await;
        let state = Arc::new(InMemoryRuntimeState::new());
        let source_state = state
            .new_workflow_run_state(Uuid::now_v7(), HashMap::new())
            .await?;

        let graph = Arc::new(example_loop_graph()?);
        let locs = &[Location::new("N2.L1.N1")?];

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

        let (exclude, truncate) = compute_exclude_and_truncate(
            &asset_storage_registry,
            &source_state,
            &graph,
            locs.iter(),
        )
        .await?;

        dbg!(&exclude, &truncate);
        assert_eq!(exclude.len(), 4);
        assert!(exclude.contains(&Location::new("N2.L1.N1")?));
        assert!(exclude.contains(&Location::new("N2.L1.N0")?));
        assert!(exclude.contains(&Location::new("N2.L2")?));
        assert!(exclude.contains(&Location::new("N0")?));
        assert_eq!(truncate.len(), 1);
        assert!(truncate.contains(&Location::new("N2")?));

        Ok(())
    }

    #[tokio::test]
    async fn exclude_and_truncate_task_in_map() -> miette::Result<()> {
        let (asset_storage_registry, asset_sets, _dir) =
            test_storage_registry([json!({"value": example_task_graph()?})], []).await;
        let state = Arc::new(InMemoryRuntimeState::new());
        let source_state = state
            .new_workflow_run_state(Uuid::now_v7(), HashMap::new())
            .await?;

        let graph = Arc::new(example_map_graph()?);
        let locs = &[Location::new("N2.M0.N1")?, Location::new("N2.M2.N1")?];

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

        let (exclude, truncate) = compute_exclude_and_truncate(
            &asset_storage_registry,
            &source_state,
            &graph,
            locs.iter(),
        )
        .await?;

        assert_eq!(exclude.len(), 5);
        assert!(exclude.contains(&Location::new("N2.M0.N1")?));
        assert!(exclude.contains(&Location::new("N2.M0.N0")?));
        assert!(exclude.contains(&Location::new("N2.M2.N1")?));
        assert!(exclude.contains(&Location::new("N2.M2.N0")?));
        assert!(exclude.contains(&Location::new("N0")?));
        assert_eq!(truncate.len(), 1);
        assert!(truncate.contains(&Location::new("N2")?));

        Ok(())
    }
}
