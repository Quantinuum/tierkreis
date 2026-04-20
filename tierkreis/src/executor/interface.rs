/*!
This module defines the interface contracts that the various [Executor]
implementations must satify.
*/
use std::collections::{HashMap, HashSet};

use futures::future::BoxFuture;
use futures::stream::BoxStream;
use serde_json::Value;

use crate::asset_storage::interface::AssetSpec;
use crate::event::Event;

/// [`TaskPlan`] describes how a Task should be executed on an Executor.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct TaskPlan {
    /// The name of the Worker to invoke.
    pub worker_name: String,
    /// The name of the Worker's task to invoke.
    pub task_name: String,
    /// The names and locations of the inputs to the task.
    pub inputs: HashMap<String, AssetSpec>,
    /// The names of the outputs of the task.
    // TODO: We can find this out from the workers
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
    fn execute(&self, task_plans: Vec<TaskPlan>) -> BoxFuture<'_, miette::Result<Vec<u32>>>;
    /// Listen to a stream of [Event]s from the [Executor] about changes in Task state.
    ///
    /// Typically this method should only be called once and the stream should be only
    /// end once the [Executor] is dropped.
    fn listen(&self) -> miette::Result<BoxStream<'_, Event>>;
    /// Signal that the Tasks with the specified ids should be cancelled when possible.
    ///
    /// There is no guarantee that the Tasks will not run, but the [Executor] should
    /// make best-effort to avoid running them if possible.
    fn cancel(&self, task_ids: Vec<u32>) -> miette::Result<()>;
}
