/*!
This module defines the interface contracts that the various [`RuntimeState`]
and [`WorkflowRunState`] implementations must satisfy.
*/
use std::{collections::HashMap, fmt::Debug};

use bitvec::vec::BitVec;
use chrono::{DateTime, Utc};
use tokio::sync::watch;
use uuid::Uuid;

use crate::{asset_storage::AssetSpec, event::Event, graph::WorkflowGraph, location::Location};

/// [`RunAttemptUpdated`] is a struct that is emitted by the [`RuntimeState`] interface
/// whenever a run attempt changes, in order to drive further workflow orchestration.
#[derive(Debug, Clone, PartialEq)]
pub struct RunAttemptUpdated {
    /// The unique identifier of the run.
    pub run_id: Uuid,
    /// The number of the run attempt, typically sequential.
    pub attempt: u32,
    /// Whether the workflow is complete/cancelled/errored and no further orchestration is required.
    pub stopped: bool,
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
}

/// [`RuntimeState`] is an interface to the state of the overall tierkreis runtime, across
/// all of the running and completed Workflows.
pub trait RuntimeState: Debug + Send + Sync {
    /// [`WorkflowRunState`] is the implementation of the [`WorkflowRunState`] trait that is associated
    /// with this [`RuntimeState`] implementation and returned by the `workflow_run_state` method.
    type WorkflowRunState: WorkflowRunState;

    /// Retrieve the [`WorkflowGraph`] specified by id.
    fn load_workflow(
        &self,
        workflow_id: Uuid,
    ) -> impl Future<Output = miette::Result<WorkflowGraph>> + Send;
    /// Save a [`WorkflowGraph`] and return a new id.
    fn save_workflow(
        &self,
        workflow_graph: WorkflowGraph,
    ) -> impl Future<Output = miette::Result<Uuid>> + Send;

    /// Create a new [`WorkflowRunState`] for a Workflow in the [`RuntimeState`] specified by id.
    fn new_workflow_run_state(
        &self,
        workflow_id: Uuid,
        inputs: HashMap<String, AssetSpec>,
    ) -> impl Future<Output = miette::Result<Self::WorkflowRunState>> + Send;
    /// Retrieve a handle to a [`WorkflowRunState`] depending on the `run_id` and attempt number.
    ///
    /// If the backing data for the [`WorkflowRunState`] does not exist, create it.
    fn load_workflow_run_state(
        &self,
        run_id: Uuid,
        attempt: u32,
    ) -> impl Future<Output = miette::Result<Self::WorkflowRunState>> + Send;
    /// Listen for updates about *all* of the running workflows.
    ///
    /// # Errors
    ///
    /// Will return Err if the method has already been called.
    fn listen(&self) -> miette::Result<watch::Receiver<RunAttemptUpdated>>;
}

/// [`WorkflowRunState`] is an interface to the state of an individual Workflow run attempt.
pub trait WorkflowRunState: Debug + Send + Sync {
    /// Retrieve the id for the [`WorkflowGraph`] associated with this Workflow run attempt.
    fn workflow_id(&self) -> Uuid;
    /// Retrieve the `run_id` associated with this `WorkflowRunState`.
    fn run_id(&self) -> Uuid;
    /// Retrieve the `attempt` associated with this `WorkflowRunState`.
    fn attempt(&self) -> u32;
    /// Retrieve the workflow inputs associated with this `WorkflowRunState`.
    fn load_inputs(
        &self,
    ) -> impl Future<Output = miette::Result<HashMap<String, AssetSpec>>> + Send;
    /// Update the [`WorkflowRunState`] from a [`WorkflowRunEvent`].
    fn write(&self, event: Event) -> impl Future<Output = miette::Result<()>> + Send;
    /// Read the state of a Node at the specified [`Location`].
    ///
    /// If the `location` has no existing state, a default [`NodeState`] will be returned.
    fn read(&self, location: &Location) -> impl Future<Output = miette::Result<NodeState>> + Send;
    /// Add metadata for the Workflow run. The new metadata will be merged with the existing values.
    fn add_metadata(
        &self,
        metadata: HashMap<String, String>,
    ) -> impl Future<Output = miette::Result<()>> + Send;
    /// Read the metadata for the Workflow run.
    fn read_metadata(&self)
    -> impl Future<Output = miette::Result<HashMap<String, String>>> + Send;
}
