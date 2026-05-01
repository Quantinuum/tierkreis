/*!
This module defines the event format that is emitted by [Executor][crate::executor::Executor] instances
and the [Orchestrator][crate::orchestrator::Orchestrator] that is used to build up the execution
state of the Workflow so it can be monitored and restarted.
*/
use std::{collections::HashMap, hash::RandomState};

use futures::{SinkExt, channel::mpsc};
use miette::miette;

use crate::{asset_storage::interface::AssetSpec, location::Location};

/// [`Event`] messages correspond to an update in the state of a Workflow run.
#[derive(Clone, Debug, PartialEq)]
pub enum Event {
    /// An event relating to the overall workflow.
    WorkflowRun(WorkflowRunEvent),
    /// An event relating to a specific node with a [`Location`].
    Node(NodeEvent),
}

impl Event {
    /// Returns true if the event indicates that the workflow is finished running.
    #[must_use]
    pub fn is_workflow_finished(&self) -> bool {
        match self {
            Event::WorkflowRun(run_event) => run_event.is_workflow_finished(),
            Event::Node(_) => false,
        }
    }

    /// Returns true if the event indicates that a node has some specific outputs.
    #[must_use]
    pub fn outputs(self) -> Option<HashMap<String, AssetSpec>> {
        match self {
            Event::Node(node_event) => node_event.outputs(),
            Event::WorkflowRun(_) => None,
        }
    }
}

/// [`WorkflowRunEvent`] messages relate to updates in the progress of the overall workflow
/// run.
#[derive(Clone, Debug, PartialEq)]
pub enum WorkflowRunEvent {
    /// The workflow run has started.
    Started {},
    /// The workflow run has been cancelled.
    Cancelled {},
    /// The workflow run has errored during running.
    Errored {},
    /// The workflow run has completed successfully.
    Completed {},
}

impl WorkflowRunEvent {
    /// Returns `true` if the workflow has finished running
    #[must_use]
    pub fn is_workflow_finished(&self) -> bool {
        matches!(
            self,
            WorkflowRunEvent::Cancelled {}
                | WorkflowRunEvent::Errored {}
                | WorkflowRunEvent::Completed {}
        )
    }
}

/// [`NodeEvent`] messages are emitted from [Executor][crate::executor::Executor] instances and the
/// [Orchestrator][crate::orchestrator::Orchestrator].
///
/// [`NodeEvent`] messages correspond to an update in the state of a Node during the Workflow
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

/// [`RunningStateUpdate`] contains extra information about how the state of
/// a Running node should be updated to track the progress of the workflow run.
#[derive(Clone, Debug, PartialEq)]
pub enum RunningStateUpdate {
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
    /// The node is "looping" with the specified index.
    ///
    /// This state is triggered by a `Loop` node when it is ready to begin
    /// the next iteration.
    Looping {
        /// The index of the next iteration to perform.
        index: u32,
    },
    /// The node is "mapping" over a data structure of a specified size.
    ///
    /// This state is triggered by a `Map` node when it begins running.
    MapStarted {
        /// The number of elements in the data structure to be mapped.
        size: u32,
    },
    /// An output for a `Map` node is ready at a specific index.
    MapElemComplete {
        /// The index in the data structure that has completed.
        index: u32,
    },
}

/// [`NodeStatus`] defines the various states that Nodes can be in.
#[derive(Clone, Debug, PartialEq)]
pub enum NodeStatus {
    /// The node is scheduled to be run by the [Orchestrator].
    Scheduled,
    /// The node is queued to run using an [Executor][crate::executor::Executor].
    Queued,
    /// The node is running on an [Executor][crate::executor::Executor].
    Running {
        /// An update to the state of the Node to apply.
        state_update: Option<RunningStateUpdate>,
    },
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
            status: NodeStatus::Running { state_update: None },
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
                state_update: Some(RunningStateUpdate::Switching { cond }),
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
    index: u32,
) -> miette::Result<()> {
    event_sender
        .send(Event::Node(NodeEvent {
            loc: loc.clone(),
            status: NodeStatus::Running {
                state_update: Some(RunningStateUpdate::Looping { index }),
            },
        }))
        .await
        .map_err(|err| {
            miette!("Failed to send running loop event: {err}")
                .wrap_err(miette!("At location: {loc:?}"))
        })
}

/// Utility function to send a new [`Event`] with [`Status::Running`] and the number of
/// elements of the data structure the Map node is being applied to.
///
/// # Errors
///
/// Will return Err if the channel for `event_sender` is full or closed.
pub async fn send_running_map(
    event_sender: &mut EventSender,
    loc: Location,
    size: u32,
) -> miette::Result<()> {
    event_sender
        .send(Event::Node(NodeEvent {
            loc: loc.clone(),
            status: NodeStatus::Running {
                state_update: Some(RunningStateUpdate::MapStarted { size }),
            },
        }))
        .await
        .map_err(|err| {
            miette!("Failed to send running map event: {err}")
                .wrap_err(miette!("At location: {loc:?}"))
        })
}

/// Utility function to send a new [`Event`] with [`Status::Running`] and index of
/// the data structure the Map node is being applied to that has finished.
///
/// # Errors
///
/// Will return Err if the channel for `event_sender` is full or closed.
pub async fn send_map_elem_complete(
    event_sender: &mut EventSender,
    loc: Location,
    index: u32,
) -> miette::Result<()> {
    event_sender
        .send(Event::Node(NodeEvent {
            loc: loc.clone(),
            status: NodeStatus::Running {
                state_update: Some(RunningStateUpdate::MapElemComplete { index }),
            },
        }))
        .await
        .map_err(|err| {
            miette!("Failed to send running map event: {err}")
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
        .send(Event::WorkflowRun(WorkflowRunEvent::Completed {}))
        .await
        .map_err(|err| miette!("Failed to send workflow run complete event: {err}"))
}
