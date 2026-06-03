/*!
This module defines the event format that is emitted by [Executor][crate::executor::Executor] instances
and the [Orchestrator][crate::orchestrator::Orchestrator] that is used to build up the execution
state of the Workflow so it can be monitored and restarted.
*/
use std::{collections::HashMap, hash::RandomState};

use futures::{SinkExt, channel::mpsc};
use miette::miette;

use crate::{asset_storage::interface::AssetSpec, location::Location};

#[derive(Clone, Debug, PartialEq)]
pub enum Event {
    WorkflowRun(WorkflowRunEvent),
    Node(NodeEvent),
}

impl Event {
    #[must_use]
    pub fn is_workflow_finished(&self) -> bool {
        match self {
            Event::WorkflowRun(run_event) => run_event.is_workflow_finished(),
            _ => false,
        }
    }

    #[must_use]
    pub fn outputs(self) -> Option<HashMap<String, AssetSpec>> {
        match self {
            Event::Node(node_event) => node_event.outputs(),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum WorkflowRunEvent {
    WorkflowStarted {},
    WorkflowCancelled {},
    WorkflowErrored {},
    WorkflowCompleted {},
}

impl WorkflowRunEvent {
    /// Returns `true` if the workflow has finished running
    #[must_use]
    pub fn is_workflow_finished(&self) -> bool {
        matches!(
            self,
            WorkflowRunEvent::WorkflowCancelled {}
                | WorkflowRunEvent::WorkflowErrored {}
                | WorkflowRunEvent::WorkflowCompleted {}
        )
    }
}

/// [NodeEvent] messages are emitted from [Executor][crate::executor::Executor] instances and the
/// [Orchestrator][crate::orchestrator::Orchestrator].
///
/// [Event] messages correspond to an update in the state of a Node during the Workflow
/// execution.
#[derive(Clone, Debug, PartialEq)]
pub struct NodeEvent {
    /// The location of the Node for this Event.
    pub loc: Location,
    /// The new status of the Node.
    pub status: NodeStatus,
}

impl NodeEvent {
    /// Returns `true` if the status field of the event is [`Status::Complete`]
    #[must_use]
    pub fn is_node_complete(&self) -> bool {
        matches!(self.status, NodeStatus::Complete { .. })
    }

    /// Returns the `outputs` field of a [`Status::Complete`] `status` and
    /// None if the `status` field is any other variant.
    #[must_use]
    pub fn outputs(self) -> Option<HashMap<String, AssetSpec>> {
        match self.status {
            NodeStatus::Complete { outputs, .. } => Some(outputs),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum RunningState {
    /// The node is "switching" and will resolve when the corresponding
    /// condition branch resolves.
    ///
    /// This state is triggered by an `IfElse` node when a value
    /// appears on a `pred` port and the corresponding branches
    /// are still resolving.
    Switching {
        /// The value that the `pred` port resolved to.
        cond: bool,
    },
    Looping {
        loop_index: u32,
    },
}

/// [Status] defines the various states that Nodes can be in.
#[derive(Clone, Debug, PartialEq)]
pub enum NodeStatus {
    /// The node is scheduled to be run by the [Orchestrator].
    Scheduled,
    /// The node is queued to run using an [Executor][crate::executor::Executor].
    Queued,
    /// The node is running on an [Executor][crate::executor::Executor].
    Running { state: Option<RunningState> },
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

/// [`EventSender`] is an multi-producer single-consumer producer for [`Event`] messages.
///
/// This alias is useful for [Executor][`crate::executor::Executor`] implementors who
/// wish to produce [Event] messages that can be consumed by an [`EventReceiver`].
pub type EventSender = mpsc::Sender<Event>;

/// [`EventReceiver`] is an multi-producer single-consumer consumer for [Event] messages.
///
/// This alias is useful for [Executor][`crate::executor::Executor`] implementors who
/// wish to forward [`Event`] messages as this type implements [`Stream`].
pub type EventReceiver = mpsc::Receiver<Event>;

/// Utility function to send a new [`Event`] with [`Status::Running`].
///
/// # Errors
///
/// Will return Err if the channel for `event_sender` is full or closed.
pub async fn send_running(event_sender: &mut EventSender, loc: Location) -> miette::Result<()> {
    event_sender
        .send(Event::Node(NodeEvent {
            loc,
            status: NodeStatus::Running { state: None },
        }))
        .await
        .map_err(|err| miette!("Failed to send running event: {err}"))
}

/// Utility function to send a new [`Event`] with [`Status::Cancelled`].
///
/// # Errors
///
/// Will return Err if the channel for `event_sender` is full or closed.
pub async fn send_cancelled(event_sender: &mut EventSender, loc: Location) -> miette::Result<()> {
    event_sender
        .send(Event::Node(NodeEvent {
            loc,
            status: NodeStatus::Cancelled {},
        }))
        .await
        .map_err(|err| miette!("Failed to send cancelled event: {err}"))
}

/// Utility function to send a new [`Event`] with [`Status::Complete`] and output [`AssetSpec`]s.
///
/// # Errors
///
/// Will return Err if the channel for `event_sender` is full or closed.
pub async fn send_complete(
    event_sender: &mut EventSender,
    loc: Location,
    outputs: HashMap<String, AssetSpec, RandomState>,
) -> miette::Result<()> {
    event_sender
        .send(Event::Node(NodeEvent {
            loc: loc.clone(),
            status: NodeStatus::Complete { outputs },
        }))
        .await
        .map_err(|err| {
            miette!("Failed to send complete event: {err}")
                .wrap_err(miette!("At location: {loc:?}"))
        })
}

/// Utility function to send a new [`Event`] with [`Status::Running`] and a conditional value
/// for how the switch should resolve.
///
/// # Errors
///
/// Will return Err if the channel for `event_sender` is full or closed.
pub async fn send_running_switching(
    event_sender: &mut EventSender,
    loc: Location,
    cond: bool,
) -> miette::Result<()> {
    event_sender
        .send(Event::Node(NodeEvent {
            loc: loc.clone(),
            status: NodeStatus::Running {
                state: Some(RunningState::Switching { cond }),
            },
        }))
        .await
        .map_err(|err| {
            miette!("Failed to send switching event: {err}")
                .wrap_err(miette!("At location: {loc:?}"))
        })
}

/// Utility function to send a new [`Event`] with [`Status::Running`] and a loop index value
/// for the current iteration index of the loop
///
/// # Errors
///
/// Will return Err if the channel for `event_sender` is full or closed.
pub async fn send_running_loop(
    event_sender: &mut EventSender,
    loc: Location,
    loop_index: u32,
) -> miette::Result<()> {
    event_sender
        .send(Event::Node(NodeEvent {
            loc: loc.clone(),
            status: NodeStatus::Running {
                state: Some(RunningState::Looping { loop_index }),
            },
        }))
        .await
        .map_err(|err| {
            miette!("Failed to send switching event: {err}")
                .wrap_err(miette!("At location: {loc:?}"))
        })
}

/// Utility function to send a new [`Event`] with [`Status::Error`] and an error message.
///
/// # Errors
///
/// Will return Err if the channel for `event_sender` is full or closed.
pub async fn send_error(
    event_sender: &mut EventSender,
    loc: Location,
    err: &miette::Error,
) -> miette::Result<()> {
    event_sender
        .send(Event::Node(NodeEvent {
            loc,
            status: NodeStatus::Error {
                error: err.to_string(),
                detail: None,
            },
        }))
        .await
        .map_err(|err| miette!("Failed to send error event: {err}"))
}

/// Utility function to send a new [`Event`].
///
/// # Errors
///
/// Will return Err if the channel for `event_sender` is full or closed.
pub async fn send_workflow_run_complete(event_sender: &mut EventSender) -> miette::Result<()> {
    event_sender
        .send(Event::WorkflowRun(WorkflowRunEvent::WorkflowCompleted {}))
        .await
        .map_err(|err| miette!("Failed to send workflow run complete event: {err}"))
}
