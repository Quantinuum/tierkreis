/*!
This module defines the event format that is emitted by [Executor][crate::executor::Executor] instances
and the [Orchestrator][crate::orchestrator::Orchestrator] that is used to build up the execution
state of the Workflow so it can be monitored and restarted.
*/
use std::{collections::HashMap, hash::RandomState};

use bitvec::vec::BitVec;
use futures::{SinkExt, channel::mpsc};
use miette::{Context, IntoDiagnostic};
use uuid::Uuid;

use crate::{asset_storage::interface::AssetSpec, location::Location};

/// [`RuntimeEvent`] messages correspond to an update in the Runtime.
#[derive(Clone, Debug, PartialEq)]
pub enum RuntimeEvent {
    /// An event relating to a specific workflow run.
    WorkflowRun {
        /// The id of the workflow run.
        workflow_run_id: Uuid,
        /// The current attempt of the workflow run.
        attempt: u32,
        /// The event payload.
        event: WorkflowRunEvent,
    },
}

impl RuntimeEvent {
    /// Returns true if the event indicates that the workflow is finished running.
    #[must_use]
    pub fn is_workflow_finished(&self) -> bool {
        match self {
            RuntimeEvent::WorkflowRun { event, .. } => event.is_workflow_finished(),
        }
    }

    /// Returns the outputs from locations specified in the event if any.
    #[must_use]
    pub fn outputs(self) -> Vec<HashMap<String, AssetSpec>> {
        match self {
            RuntimeEvent::WorkflowRun {
                event: WorkflowRunEvent::NodeEvent(node_event),
                ..
            } => node_event.outputs(),
            RuntimeEvent::WorkflowRun { .. } => Vec::new(),
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
    Completed {
        // TODO: Should this contain outputs?
        // if so also update the outputs function
    },
    /// The workflow run is waiting to be executed.
    Queued {},
    /// An event relating to a specific set of nodes in the workflow run.
    NodeEvent(NodeEvent),
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
    pub locs: Vec<Location>,
    /// The new status of the Node.
    pub status: NodeStatus,
}

impl NodeEvent {
    /// Returns `true` if the status field of the event is [`Status::Complete`]
    #[must_use]
    pub fn is_node_complete(&self) -> bool {
        matches!(self.status, NodeStatus::Complete { .. })
    }

    /// Returns the outputs from locations specified in the event if any.
    #[must_use]
    pub fn outputs(self) -> Vec<HashMap<String, AssetSpec>> {
        match self.status {
            NodeStatus::Complete { outputs, .. } => outputs,
            _ => Vec::new(),
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
        bits: BitVec<u8>,
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
        /// The outputs from the nodes
        outputs: Vec<HashMap<String, AssetSpec>>,
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

    /// The state of the node is unknown.
    Unknown,
}

/// [`EventSender`] is an multi-producer single-consumer producer for [`Event`] messages.
///
/// This alias is useful for [Executor][`crate::executor::Executor`] implementors who
/// wish to produce [Event] messages that can be consumed by an [`EventReceiver`].
pub type EventSender = mpsc::Sender<RuntimeEvent>;

/// [`EventReceiver`] is an multi-producer single-consumer consumer for [Event] messages.
///
/// This alias is useful for [Executor][`crate::executor::Executor`] implementors who
/// wish to forward [`Event`] messages as this type implements [`Stream`].
pub type EventReceiver = mpsc::Receiver<RuntimeEvent>;

/// Utility function to send a new [`Event`] with [`NodeStatus::Running`].
///
/// # Errors
///
/// Will return Err if the channel for `event_sender` is full or closed.
pub async fn send_running(
    event_sender: &mut EventSender,
    workflow_run_id: Uuid,
    attempt: u32,
    loc: Location,
) -> miette::Result<()> {
    let event = RuntimeEvent::WorkflowRun {
        workflow_run_id,
        attempt,
        event: WorkflowRunEvent::NodeEvent(NodeEvent {
            locs: vec![loc],
            status: NodeStatus::Running { state_update: None },
        }),
    };
    event_sender
        .send(event)
        .await
        .into_diagnostic()
        .wrap_err("Failed to send node running event")
}

/// Utility function to send a new [`Event`] with [`NodeStatus::Cancelled`].
///
/// # Errors
///
/// Will return Err if the channel for `event_sender` is full or closed.
pub async fn send_cancelled(
    event_sender: &mut EventSender,
    workflow_run_id: Uuid,
    attempt: u32,
    loc: Location,
) -> miette::Result<()> {
    let event = RuntimeEvent::WorkflowRun {
        workflow_run_id,
        attempt,
        event: WorkflowRunEvent::NodeEvent(NodeEvent {
            locs: vec![loc],
            status: NodeStatus::Cancelled,
        }),
    };
    event_sender
        .send(event)
        .await
        .into_diagnostic()
        .wrap_err("Failed to send node cancelled event")
}

/// Utility function to send a new [`Event`] with [`NodeStatus::Complete`] and output [`AssetSpec`]s.
///
/// # Errors
///
/// Will return Err if the channel for `event_sender` is full or closed.
pub async fn send_complete(
    event_sender: &mut EventSender,
    workflow_run_id: Uuid,
    attempt: u32,
    locs: Vec<Location>,
    outputs: Vec<HashMap<String, AssetSpec, RandomState>>,
) -> miette::Result<()> {
    let event = RuntimeEvent::WorkflowRun {
        workflow_run_id,
        attempt,
        event: WorkflowRunEvent::NodeEvent(NodeEvent {
            locs,
            status: NodeStatus::Complete { outputs },
        }),
    };
    event_sender
        .send(event)
        .await
        .into_diagnostic()
        .wrap_err("Failed to send node complete event")
}

/// Utility function to send a new [`Event`] with [`NodeStatus::Running`] and a conditional value
/// for how the switch should resolve.
///
/// # Errors
///
/// Will return Err if the channel for `event_sender` is full or closed.
pub async fn send_running_switching(
    event_sender: &mut EventSender,
    workflow_run_id: Uuid,
    attempt: u32,
    loc: Location,
    cond: bool,
) -> miette::Result<()> {
    let event = RuntimeEvent::WorkflowRun {
        workflow_run_id,
        attempt,
        event: WorkflowRunEvent::NodeEvent(NodeEvent {
            locs: vec![loc],
            status: NodeStatus::Running {
                state_update: Some(RunningStateUpdate::Switching { cond }),
            },
        }),
    };
    event_sender
        .send(event)
        .await
        .into_diagnostic()
        .wrap_err("Failed to send running switch node event")
}

/// Utility function to send a new [`Event`] with [`NodeStatus::Running`] and a loop index value
/// for the current iteration index of the loop
///
/// # Errors
///
/// Will return Err if the channel for `event_sender` is full or closed.
pub async fn send_running_loop(
    event_sender: &mut EventSender,
    workflow_run_id: Uuid,
    attempt: u32,
    loc: Location,
    index: u32,
) -> miette::Result<()> {
    let event = RuntimeEvent::WorkflowRun {
        workflow_run_id,
        attempt,
        event: WorkflowRunEvent::NodeEvent(NodeEvent {
            locs: vec![loc],
            status: NodeStatus::Running {
                state_update: Some(RunningStateUpdate::Looping { index }),
            },
        }),
    };
    event_sender
        .send(event)
        .await
        .into_diagnostic()
        .wrap_err("Failed to send running loop node event")
}

/// Utility function to send a new [`Event`] with [`NodeStatus::Running`] and the number of
/// elements of the data structure the Map node is being applied to.
///
/// # Errors
///
/// Will return Err if the channel for `event_sender` is full or closed or if the
/// size of the map is larger than `u32`.
pub async fn send_running_map(
    event_sender: &mut EventSender,
    workflow_run_id: Uuid,
    attempt: u32,
    loc: Location,
    size: usize,
) -> miette::Result<()> {
    let event = RuntimeEvent::WorkflowRun {
        workflow_run_id,
        attempt,
        event: WorkflowRunEvent::NodeEvent(NodeEvent {
            locs: vec![loc],
            status: NodeStatus::Running {
                state_update: Some(RunningStateUpdate::MapStarted {
                    size: u32::try_from(size).into_diagnostic()?,
                }),
            },
        }),
    };
    event_sender
        .send(event)
        .await
        .into_diagnostic()
        .wrap_err("Failed to send running map node event")
}

/// Utility function to send a new [`Event`] with [`NodeStatus::Running`] and index of
/// the data structure the Map node is being applied to that has finished.
///
/// # Errors
///
/// Will return Err if the channel for `event_sender` is full or closed or if the
/// index of the map is larger than `u32`.
pub async fn send_map_elem_complete(
    event_sender: &mut EventSender,
    workflow_run_id: Uuid,
    attempt: u32,
    loc: Location,
    bits: BitVec<u8>,
) -> miette::Result<()> {
    let event = RuntimeEvent::WorkflowRun {
        workflow_run_id,
        attempt,
        event: WorkflowRunEvent::NodeEvent(NodeEvent {
            locs: vec![loc.clone()],
            status: NodeStatus::Running {
                state_update: Some(RunningStateUpdate::MapElemComplete { bits }),
            },
        }),
    };
    event_sender
        .send(event)
        .await
        .into_diagnostic()
        .wrap_err("Failed to send map node element complete event")
}

/// Utility function to send a new [`Event`] with [`NodeStatus::Error`] and an error message.
///
/// # Errors
///
/// Will return Err if the channel for `event_sender` is full or closed.
pub async fn send_error(
    event_sender: &mut EventSender,
    workflow_run_id: Uuid,
    attempt: u32,
    loc: Location,
    err: &miette::Error,
) -> miette::Result<()> {
    let err_chain: Vec<_> = err.chain().map(ToString::to_string).collect();
    let detail = err_chain.join("\n\nWhich was caused by:\n\n");

    let event = RuntimeEvent::WorkflowRun {
        workflow_run_id,
        attempt,
        event: WorkflowRunEvent::NodeEvent(NodeEvent {
            locs: vec![loc],
            status: NodeStatus::Error {
                error: err.to_string(),
                detail: Some(detail),
            },
        }),
    };
    event_sender
        .send(event)
        .await
        .into_diagnostic()
        .wrap_err("Failed to send node error event")
}

/// Utility function to send a new [`Event`] with [`WorkflowRunEvent::Completed`].
///
/// # Errors
///
/// Will return Err if the channel for `event_sender` is full or closed.
pub async fn send_workflow_run_complete(
    event_sender: &mut EventSender,
    workflow_run_id: Uuid,
    attempt: u32,
) -> miette::Result<()> {
    let event = RuntimeEvent::WorkflowRun {
        workflow_run_id,
        attempt,
        event: WorkflowRunEvent::Completed {},
    };
    event_sender
        .send(event)
        .await
        .into_diagnostic()
        .wrap_err("Failed to send workflow complete event")
}

/// Utility function to send a new [`Event`] with [`WorkflowRunEvent::Queued`].
///
/// # Errors
///
/// Will return Err if the channel for `event_sender` is full or closed.
pub async fn send_workflow_run_queued(
    event_sender: &mut EventSender,
    workflow_run_id: Uuid,
    attempt: u32,
) -> miette::Result<()> {
    let event = RuntimeEvent::WorkflowRun {
        workflow_run_id,
        attempt,
        event: WorkflowRunEvent::Queued {},
    };
    event_sender
        .send(event)
        .await
        .into_diagnostic()
        .wrap_err("Failed to send workflow queued event")
}
#[cfg(test)]
mod tests {
    use super::*;

    use miette::miette;

    // Test that we populate the detail field of error events
    // with a reasonable chain.
    #[tokio::test]
    async fn error_chain_formatting() -> miette::Result<()> {
        let mut err = miette!("Root cause");
        err = err.wrap_err("First Context");
        err = err.wrap_err("Second Context");

        let (mut send, mut recv) = mpsc::channel(8);
        send_error(&mut send, Uuid::nil(), 0, Location::root(), &err).await?;

        let event = recv.recv().await.into_diagnostic()?;
        dbg!(&event);
        assert!(matches!(
            event,
            RuntimeEvent::WorkflowRun {
                event: WorkflowRunEvent::NodeEvent(NodeEvent {
                    status: NodeStatus::Error { ref error, ref detail },
                    ..
                }),
                ..
            } if error == "Second Context"
                && *detail == Some("Second Context

Which was caused by:

First Context

Which was caused by:

Root cause".to_string())
        ));

        Ok(())
    }
}
