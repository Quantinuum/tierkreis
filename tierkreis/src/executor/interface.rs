/*!
This module defines the interface contracts that the various [Executor]
implementations must satisfy.
*/
use std::collections::{HashMap, HashSet};

use futures::future::BoxFuture;
use futures::stream::BoxStream;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

use crate::asset_storage::interface::AssetSpec;
use crate::event::{NodeStatus, RuntimeEvent};
use crate::location::Location;

/// A node location and its recovered status.
pub type UniqueLocState = (Uuid, u32, Location, NodeStatus);
/// A node location and the persisted handle used to restore it.
pub type UniqueNodeHandle = (Uuid, u32, Location, TaskHandle);

/// [`TaskPlan`] describes how a Task should be executed on an Executor.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct TaskPlan {
    /// The workflow run id that this Task was created for.
    pub workflow_run_id: Uuid,
    /// The attempt number of the workflow run that this task was created for.
    pub attempt: u32,
    /// The location of the Node which this Task was created for.
    pub loc: Location,
    /// The name of the Worker to invoke.
    pub worker_name: String,
    /// The name of the Worker's task to invoke.
    pub task_name: String,
    /// The names and locations of the inputs to the task.
    pub inputs: HashMap<String, AssetSpec>,
    /// The names of the outputs of the task.
    pub outputs: HashSet<String>,
    /// A specific [`AssetStorage`][crate::asset_storage::AssetStorage]
    /// from an [`AssetStorageRegistry`][crate::asset_storage::AssetStorageRegistry]
    /// to save outputs to.
    ///
    /// If not specified the [Executor] will save the outputs to a default
    /// [`AssetStorage`][crate::asset_storage::AssetStorage] instead.
    pub output_storage_name: Option<String>,
    /// An arbitrary Resource specification for the Task. [Executor]s
    /// should validate this and convert it to a usable representation.
    pub resources: HashMap<String, Value>,
    /// An arbitrary Environment specification for the Task. [Executor]s
    /// should validate this and convert it to a usable representation.
    pub environment: HashMap<String, Value>,

    /// An executor-specific identifier for a task that can be persisted and restored.
    /// Obligation of the [Executor] to check this when a task is received.
    pub task_handle: Option<TaskHandle>,
}

/// An executor-specific identifier for a task that can be persisted and restored.
/// None indicates Inmemory/needs to be scheduled
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "executor", content = "task")]
pub enum TaskHandle {
    /// A task running as a subprocess.
    Subprocess {
        /// The process identifier.
        pid: u32,
        /// The process start time in Linux clock ticks.
        start_time: u64,
    },
    /// A task running on nexus.
    Nexus {
        /// The job identifier assigned by Nexus.
        job_id: String,
    },
}

/// [`WorkerSpec`] defines the information about a Worker returned by an
/// [Executor] from the [`Executor::workers`] method.
#[derive(Debug, PartialEq)]
pub struct WorkerSpec {
    /// The name of the Worker.
    pub worker_name: String,
}

/// The [Executor] defines the minimum methods required for Task execution.
pub trait Executor: Send + Sync {
    /// Return a list of the Workers available to the Executor with their metadata.
    fn workers(&self) -> BoxFuture<'_, miette::Result<Vec<WorkerSpec>>>;
    /// Dispatch a list of [`TaskPlan`]s to be run on an [Executor], returning a list of Task
    /// ids that can be used to cancel the Tasks if needed.
    ///
    /// This method does not block and updates about the Tasks will appear in the
    /// stream provided by the [`Executor::listen`] method.
    /// This needs to handle the case when a Task has already started.
    fn execute(&self, task_plans: Vec<TaskPlan>) -> BoxFuture<'_, miette::Result<()>>;
    /// Listen to a stream of [Event]s from the [Executor] about changes in Task state.
    ///
    /// Typically this method should only be called once and the stream should be only
    /// end once the [Executor] is dropped.
    ///
    /// # Errors
    ///
    /// Will return Err if the method has already been called.
    fn listen(&self) -> miette::Result<BoxStream<'static, RuntimeEvent>>;
    /// Signal that the Tasks with the specified ids should be cancelled when possible.
    ///
    /// There is no guarantee that the Tasks will not run, but the [Executor] should
    /// make best-effort to avoid running them if possible.
    ///
    /// # Errors
    ///
    /// Will return Err if the [Executor] is unreachable.
    fn cancel(
        &self,
        workflow_run_id: Uuid,
        attempt: u32,
        task_locations: Vec<Location>,
    ) -> BoxFuture<'_, miette::Result<()>>;
}
