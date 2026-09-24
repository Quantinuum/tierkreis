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
use futures::future::BoxFuture;
use tokio::sync::watch;
use uuid::Uuid;

use crate::{
    asset_storage::AssetSpec, event::WorkflowRunEvent, executor::interface::TaskHandle,
    graph::WorkflowGraph, location::Location,
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
    /// attempt(new) = max(attempts) + 1, should always be sequential
    /// This can be a partial restart, e.g., if some nodes errored.
    fn new_attempt(&self, run_id: Uuid)
    -> BoxFuture<'_, miette::Result<Arc<dyn WorkflowRunState>>>;
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
    /// Update the [`WorkflowRunState`] from a [`WorkflowRunEvent`].
    fn write(&self, event: WorkflowRunEvent) -> BoxFuture<'_, miette::Result<()>>;
    /// Read the state of a Node at the specified [`Location`].
    ///
    /// If the `location` has no existing state, a default [`NodeState`] will be returned.
    fn read<'a>(&'a self, location: &'a Location) -> BoxFuture<'a, miette::Result<NodeState>>;
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

    /// Copy node state (and outputs) from `source` into `self`.
    ///
    /// Locations in `exclude` are skipped to reset them
    /// Locations in `truncate` retain structural state such as `cond` and `loop_index` is
    /// map progress is reset
    fn copy_node_states_from<'a>(
        &'a self,
        source: &'a dyn WorkflowRunState,
        exclude: &'a HashSet<Location>,
        truncate: &'a HashSet<Location>,
    ) -> BoxFuture<'a, miette::Result<()>>;
}
