use std::collections::{HashMap, HashSet};

use futures::future::BoxFuture;
use futures::stream::BoxStream;
use serde_json::Value;

use crate::asset_storage::interface::AssetSpec;

#[derive(Clone, Debug, PartialEq)]
pub struct Event<ID> {
    pub id: ID,
    pub status: Status,
    pub detail: Option<String>,
}

impl<ID> Event<ID> {
    pub fn is_complete(&self) -> bool {
        matches!(self.status, Status::Complete { .. })
    }

    pub fn outputs(self) -> Option<HashMap<String, AssetSpec>> {
        match self.status {
            Status::Complete { outputs } => Some(outputs),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum Status {
    Queued,
    Running,
    Complete { outputs: HashMap<String, AssetSpec> },
    Cancelled,
    Error { error: String },
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct TaskPlan {
    pub worker_name: String,
    pub task_name: String,
    pub inputs: HashMap<String, AssetSpec>,
    // TODO: We can find this out from the workers
    pub outputs: HashSet<String>,
    // Override the default from the executor.
    pub output_storage_name: Option<String>,

    pub resources: HashMap<String, Value>,
    pub environment: HashMap<String, Value>,
}

#[derive(Debug, PartialEq)]
pub struct WorkerSpec {
    pub worker_name: String,
}

pub trait Executor: Send + Sync {
    fn workers(&self) -> BoxFuture<'_, miette::Result<Vec<WorkerSpec>>>;
    fn execute(&self, task_plans: Vec<TaskPlan>) -> BoxFuture<'_, miette::Result<Vec<u32>>>;
    fn listen(&self) -> miette::Result<BoxStream<'_, Event<u32>>>;
    fn cancel(&self, task_ids: Vec<u32>) -> miette::Result<()>;
}
