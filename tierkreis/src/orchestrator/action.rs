use std::collections::{HashMap, HashSet};

use bitvec::vec::BitVec;
use serde_json::Value;

use crate::{
    asset_storage::interface::AssetSpec, executor::interface::TaskHandle, location::Location,
};

/// An operation produced by the [`ActionPlanner`](super::ActionPlanner).
#[derive(Debug, Clone, PartialEq)]
pub struct Action {
    /// The node location in the graph where orchestration should occur.
    pub loc: Location,
    /// The kind of action to perform.
    pub kind: ActionKind,
}

/// An operation that advances workflow execution or updates workflow state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ActionKind {
    /// A task to submit to an executor.
    PerformTask {
        /// The name of the Worker to call.
        worker_name: String,
        /// The name of the Task to call.
        task_name: String,
        /// The input assets for the Task.
        inputs: HashMap<String, AssetSpec>,
        /// The names of the outputs of the Task.
        outputs: HashSet<String>,
        /// A persisted executor handle used to reattach after a restart.
        task_handle: Option<TaskHandle>,
        /// Resource requirements interpreted by the selected executor.
        resources: HashMap<String, Value>,
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
    /// Mark the overall workflow as errored.
    WorkflowErrored {},
    /// Mark the overall workflow as complete.
    WorkflowFinished {},
}

/// A task produced by graph planning before runtime-specific execution details
/// are attached.
#[derive(Debug, Clone, PartialEq)]
pub struct PlannedTask {
    pub(super) loc: Location,
    pub(super) worker_name: String,
    pub(super) task_name: String,
    pub(super) inputs: HashMap<String, AssetSpec>,
    pub(super) outputs: HashSet<String>,
    pub(super) task_handle: Option<TaskHandle>,
    pub(super) resources: HashMap<String, Value>,
}

impl PlannedTask {
    /// Location of the task node in the workflow.
    #[must_use]
    pub fn location(&self) -> &Location {
        &self.loc
    }

    /// Worker that provides the task implementation.
    #[must_use]
    pub fn worker_name(&self) -> &str {
        &self.worker_name
    }

    /// Name of the task to invoke on the worker.
    #[must_use]
    pub fn task_name(&self) -> &str {
        &self.task_name
    }

    /// Resource requirements that the selected executor will interpret and
    /// queue according to its own resource-management policy.
    #[must_use]
    pub fn resources(&self) -> &HashMap<String, Value> {
        &self.resources
    }
}

/// Tasks assigned to one executor for dispatch as a group.
#[derive(Debug, Clone, PartialEq)]
pub struct ExecutorTaskGroup {
    /// Name of the executor responsible for the tasks.
    pub executor_name: String,
    /// Tasks assigned to the executor.
    pub tasks: Vec<PlannedTask>,
}

/// A node that should be marked complete with the supplied outputs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NodeCompletion {
    /// Location of the completed node.
    pub location: Location,
    /// Outputs produced by the node.
    pub outputs: HashMap<String, AssetSpec>,
}

/// A map node that is starting execution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MapStart {
    /// Location of the map node.
    pub location: Location,
    /// Number of elements in the map.
    pub size: usize,
}

/// Elements of a map node that completed during this orchestration cycle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MapElementsCompleted {
    /// Location of the map node.
    pub location: Location,
    /// Bitmap of completed elements.
    pub completed: BitVec<u8>,
}

/// A loop node advancing to an iteration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoopUpdate {
    /// Location of the loop node.
    pub location: Location,
    /// Iteration that should run next.
    pub index: u32,
}

/// A conditional node selecting a branch.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SwitchUpdate {
    /// Location of the conditional node.
    pub location: Location,
    /// Branch condition selected by the node.
    pub condition: bool,
}

/// State updates that the runner emits before dispatching tasks.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct NodeUpdatePlan {
    /// Nodes that should be marked complete.
    pub completions: Vec<NodeCompletion>,
    /// Map nodes beginning execution.
    pub maps_started: Vec<MapStart>,
    /// Map elements completed during this cycle.
    pub map_elements_completed: Vec<MapElementsCompleted>,
    /// Loop nodes advancing to another iteration.
    pub loops: Vec<LoopUpdate>,
    /// Conditional nodes selecting a branch.
    pub switches: Vec<SwitchUpdate>,
}

/// Terminal workflow state emitted after node updates and task dispatches.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkflowOutcome {
    /// The workflow completed successfully.
    Completed,
    /// The workflow cannot make further progress because a node errored.
    Errored,
}

/// Compact diagnostic view of an [`ActionPlan`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ActionPlanSummary {
    /// Number of executor dispatch groups.
    pub executor_groups: usize,
    /// Total number of tasks across all executor groups.
    pub tasks: usize,
    /// Number of node completion updates.
    pub node_completions: usize,
    /// Number of map start updates.
    pub maps_started: usize,
    /// Number of map element completion updates.
    pub map_elements_completed: usize,
    /// Number of loop updates.
    pub loops: usize,
    /// Number of conditional branch updates.
    pub switches: usize,
    /// Terminal outcome, when present.
    pub outcome: Option<WorkflowOutcome>,
}

/// An aggregated set of actions that can be performed as one orchestration cycle.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ActionPlan {
    pub(super) executor_groups: Vec<ExecutorTaskGroup>,
    pub(super) node_updates: NodeUpdatePlan,
    pub(super) outcome: Option<WorkflowOutcome>,
}

impl ActionPlan {
    /// Tasks grouped by their assigned executor.
    #[must_use]
    pub fn executor_groups(&self) -> &[ExecutorTaskGroup] {
        &self.executor_groups
    }

    /// Node state updates to emit before task dispatch.
    #[must_use]
    pub fn node_updates(&self) -> &NodeUpdatePlan {
        &self.node_updates
    }

    /// Terminal workflow outcome, if this cycle finishes the workflow.
    #[must_use]
    pub fn outcome(&self) -> Option<WorkflowOutcome> {
        self.outcome
    }

    /// Return a compact summary suitable for logs and diagnostics.
    #[must_use]
    pub fn summary(&self) -> ActionPlanSummary {
        ActionPlanSummary {
            executor_groups: self.executor_groups.len(),
            tasks: self
                .executor_groups
                .iter()
                .map(|group| group.tasks.len())
                .sum(),
            node_completions: self.node_updates.completions.len(),
            maps_started: self.node_updates.maps_started.len(),
            map_elements_completed: self.node_updates.map_elements_completed.len(),
            loops: self.node_updates.loops.len(),
            switches: self.node_updates.switches.len(),
            outcome: self.outcome,
        }
    }
}
