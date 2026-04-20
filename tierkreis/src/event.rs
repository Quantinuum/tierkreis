/*!
This module defines the event format that is emitted by [Executor][crate::executor::Executor] instances
and the [Orchestrator][crate::orchestrator::Orchestrator] that is used to build up the execution
state of the Workflow so it can be monitored and restarted.
*/
use std::collections::HashMap;

use crate::asset_storage::interface::AssetSpec;

/// [Event] messages are emitted from [Executor][crate::executor::Executor] instances and the
/// [Orchestrator][crate::orchestrator::Orchestrator].
///
/// [Event] messages correspond to an update in the state of a Node during the Workflow
/// execution.
#[derive(Clone, Debug, PartialEq)]
pub struct Event {
    /// The id of an executor subprocess
    // TODO: This should really be the id of a node instead, probably
    // something like Loc which is programmatically determined at
    // runtime.
    pub id: u32,
    /// The new status of the Node.
    pub status: Status,
}

impl Event {
    /// Returns `true` if the status field of the event is [`Status::Complete`]
    #[must_use] 
    pub fn is_complete(&self) -> bool {
        matches!(self.status, Status::Complete { .. })
    }

    /// Returns the `outputs` field of a [`Status::Complete`] `status` and
    /// None if the `status` field is any other variant.
    #[must_use] 
    pub fn outputs(self) -> Option<HashMap<String, AssetSpec>> {
        match self.status {
            Status::Complete { outputs } => Some(outputs),
            _ => None,
        }
    }
}

/// [Status] defines the various states that Nodes can be in.
#[derive(Clone, Debug, PartialEq)]
pub enum Status {
    /// The node is queued to run using an [Executor][crate::executor::Executor].
    Queued,
    /// The node is running on an [Executor][crate::executor::Executor].
    Running,
    /// The node is finished and has outputs.
    Complete {
        /// The outputs from the node.
        outputs: HashMap<String, AssetSpec>,
    },
    /// The node has been cancelled.
    Cancelled,
    /// The node has errored.
    Error {
        /// A short error message.
        error: String,
        /// A longer detailed context about the error.
        detail: Option<String>,
    },
}
