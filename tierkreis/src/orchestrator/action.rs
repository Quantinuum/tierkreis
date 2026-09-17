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
    /// Resource requirements that the selected executor will interpret and
    /// queue according to its own resource-management policy.
    #[must_use]
    pub fn resources(&self) -> &HashMap<String, Value> {
        &self.resources
    }
}

/// An aggregated set of actions that can be performed as one orchestration
/// cycle.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ActionPlan {
    pub(super) tasks: Vec<PlannedTask>,
    pub(super) switching: Vec<(Location, bool)>,
    pub(super) looping: Vec<(Location, u32)>,
    pub(super) mapping: Vec<(Location, usize)>,
    pub(super) map_elem_complete: HashMap<Location, BitVec<u8>>,
    pub(super) node_complete: Vec<(Location, HashMap<String, AssetSpec>)>,
    pub(super) workflow_error: bool,
    pub(super) workflow_complete: bool,
}

impl ActionPlan {
    /// Tasks awaiting executor selection and dispatch.
    #[must_use]
    pub fn tasks(&self) -> &[PlannedTask] {
        &self.tasks
    }
}
