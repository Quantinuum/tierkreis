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

use crate::{
    asset_storage::interface::AssetSpec, executor::interface::TaskHandle, location::Location,
};

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
                event: WorkflowRunEvent::NodeEvents(node_events),
                ..
            } => node_events.into_iter().map(NodeEvent::outputs).collect(),
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
    /// The workflow run has paused.
    ///
    /// This is typically due to the runtime reaching a breakpoint and
    /// having no other nodes to run.
    Paused {},
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
    NodeEvents(Vec<NodeEvent>),
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
    /// The user specified name for the Node if any.
    ///
    /// Setting the value will set the Node name in the state but not setting
    /// it will not override the previous value.
    pub name: Option<String>,
    /// The new status of the Node.
    pub status: NodeStatus,
}

impl NodeEvent {
    /// Returns `true` if the status field of the event is [`Status::Complete`]
    #[must_use]
    pub fn is_node_complete(&self) -> bool {
        matches!(self.status, NodeStatus::Complete { .. })
    }

    /// Returns the outputs from location specified in the event if any.
    #[must_use]
    pub fn outputs(self) -> HashMap<String, AssetSpec> {
        match self.status {
            NodeStatus::Complete { outputs, .. } => outputs,
            _ => HashMap::new(),
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
    /// The node is "evalling" a subgraph with the specified inputs.
    ///
    /// This state is triggered by an `Eval` node when it starts to run.
    Evalling {
        /// The inputs to the `Eval` node to be persisted.
        inputs: HashMap<String, AssetSpec>,
    },
    /// The node is "looping" with the specified index.
    ///
    /// This state is triggered by a `Loop` node when it is ready to begin
    /// the next iteration.
    Looping {
        /// The index of the next iteration to perform.
        index: u32,
        /// The inputs to the `Loop` node to be persisted.
        inputs: HashMap<String, AssetSpec>,
    },
    /// The node is "mapping" over a data structure of a specified size.
    ///
    /// This state is triggered by a `Map` node when it begins running.
    MapStarted {
        /// The input sets to the `Map` node to be persisted.
        input_sets: Vec<HashMap<String, AssetSpec>>,
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
    Scheduled {
        /// Execution context selected by the orchestrator, if any.
        execution_context: Option<String>,
    },
    /// The node is queued to run using an [Executor][crate::executor::Executor].
    Queued {
        /// Unique handle to the task on the executor that is running this node.
        handle: Option<TaskHandle>,
    },
    /// The node is running on an [Executor][crate::executor::Executor].
    Running {
        /// An update to the state of the Node to apply.
        state_update: Option<RunningStateUpdate>,
    },
    /// The node is paused as a result of hitting a breakpoint.
    Paused {},
    /// The node is finished and has outputs.
    Complete {
        /// The outputs from the node
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
pub type EventSender = mpsc::Sender<RuntimeEvent>;

/// [`EventReceiver`] is an multi-producer single-consumer consumer for [Event] messages.
///
/// This alias is useful for [Executor][`crate::executor::Executor`] implementors who
/// wish to forward [`Event`] messages as this type implements [`Stream`].
pub type EventReceiver = mpsc::Receiver<RuntimeEvent>;

const MAX_NODE_EVENTS_PER_EVENT: usize = 128;

/// Send [`NodeEvent`]s in bounded batches.
///
/// # Errors
///
/// Will return Err if the channel for `event_sender` is full or closed.
pub async fn send_node_events(
    event_sender: &mut EventSender,
    workflow_run_id: Uuid,
    attempt: u32,
    node_events: Vec<NodeEvent>,
) -> miette::Result<()> {
    let mut node_events = node_events.into_iter();

    loop {
        let node_events = node_events
            .by_ref()
            .take(MAX_NODE_EVENTS_PER_EVENT)
            .collect::<Vec<_>>();
        if node_events.is_empty() {
            break;
        }

        event_sender
            .send(RuntimeEvent::WorkflowRun {
                workflow_run_id,
                attempt,
                event: WorkflowRunEvent::NodeEvents(node_events),
            })
            .await
            .into_diagnostic()
            .wrap_err("Failed to send node events")?;
    }

    Ok(())
}

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
        event: WorkflowRunEvent::NodeEvents(vec![NodeEvent {
            loc,
            status: NodeStatus::Running { state_update: None },
            name: None,
        }]),
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
        event: WorkflowRunEvent::NodeEvents(vec![NodeEvent {
            loc,
            status: NodeStatus::Cancelled,
            name: None,
        }]),
    };
    event_sender
        .send(event)
        .await
        .into_diagnostic()
        .wrap_err("Failed to send node cancelled event")
}

/// Utility function to send a new [`Event`] with [`NodeStatus::Queued`].
///
/// # Errors
///
/// Will return Err if the channel for `event_sender` is full or closed.
pub async fn send_queued(
    event_sender: &mut EventSender,
    workflow_run_id: Uuid,
    attempt: u32,
    loc: Location,
    handle: Option<TaskHandle>,
) -> miette::Result<()> {
    let event = RuntimeEvent::WorkflowRun {
        workflow_run_id,
        attempt,
        event: WorkflowRunEvent::NodeEvents(vec![NodeEvent {
            loc,
            status: NodeStatus::Queued { handle },
            name: None,
        }]),
    };
    event_sender
        .send(event)
        .await
        .into_diagnostic()
        .wrap_err("Failed to send node queued event")
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
    outputs_by_loc: Vec<(Location, HashMap<String, AssetSpec, RandomState>)>,
) -> miette::Result<()> {
    let event = RuntimeEvent::WorkflowRun {
        workflow_run_id,
        attempt,
        event: WorkflowRunEvent::NodeEvents(
            outputs_by_loc
                .into_iter()
                .map(|(loc, outputs)| NodeEvent {
                    loc,
                    status: NodeStatus::Complete { outputs },
                    name: None,
                })
                .collect(),
        ),
    };
    event_sender
        .send(event)
        .await
        .into_diagnostic()
        .wrap_err("Failed to send node complete event")
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
        event: WorkflowRunEvent::NodeEvents(vec![NodeEvent {
            loc,
            status: NodeStatus::Error {
                error: err.to_string(),
                detail: Some(detail),
            },
            name: None,
        }]),
    };
    event_sender
        .send(event)
        .await
        .into_diagnostic()
        .wrap_err("Failed to send node error event")
}

/// Utility function to send a new [`Event`] with [`WorkflowRunEvent::Paused`].
///
/// # Errors
///
/// Will return Err if the channel for `event_sender` is full or closed.
pub async fn send_workflow_run_paused(
    event_sender: &mut EventSender,
    workflow_run_id: Uuid,
    attempt: u32,
) -> miette::Result<()> {
    let event = RuntimeEvent::WorkflowRun {
        workflow_run_id,
        attempt,
        event: WorkflowRunEvent::Paused {},
    };
    event_sender
        .send(event)
        .await
        .into_diagnostic()
        .wrap_err("Failed to send workflow paused event")
}

/// Utility function to send a new [`Event`] with [`WorkflowRunEvent::Errored`].
///
/// # Errors
///
/// Will return Err if the channel for `event_sender` is full or closed.
pub async fn send_workflow_run_errored(
    event_sender: &mut EventSender,
    workflow_run_id: Uuid,
    attempt: u32,
) -> miette::Result<()> {
    let event = RuntimeEvent::WorkflowRun {
        workflow_run_id,
        attempt,
        event: WorkflowRunEvent::Errored {},
    };
    event_sender
        .send(event)
        .await
        .into_diagnostic()
        .wrap_err("Failed to send workflow errored event")
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

#[cfg(test)]
mod tests {
    use super::*;

    use std::assert_matches;

    use futures::StreamExt;
    use miette::miette;

    #[tokio::test]
    async fn node_events_are_sent_in_bounded_batches() -> miette::Result<()> {
        let (mut send, recv) = mpsc::channel(8);
        let node_events = (0..=MAX_NODE_EVENTS_PER_EVENT)
            .map(|_| NodeEvent {
                loc: Location::root(),
                name: None,
                status: NodeStatus::Paused {},
            })
            .collect();

        send_node_events(&mut send, Uuid::nil(), 0, node_events).await?;
        drop(send);

        let events = recv.collect::<Vec<_>>().await;
        assert_eq!(events.len(), 2);
        assert_matches!(
            &events[0],
            RuntimeEvent::WorkflowRun {
                event: WorkflowRunEvent::NodeEvents(node_events),
                ..
            } if node_events.len() == MAX_NODE_EVENTS_PER_EVENT
        );
        assert_matches!(
            &events[1],
            RuntimeEvent::WorkflowRun {
                event: WorkflowRunEvent::NodeEvents(node_events),
                ..
            } if node_events.len() == 1
        );

        Ok(())
    }

    #[tokio::test]
    async fn empty_node_events_are_not_sent() -> miette::Result<()> {
        let (mut send, recv) = mpsc::channel(1);

        send_node_events(&mut send, Uuid::nil(), 0, Vec::new()).await?;
        drop(send);

        assert_eq!(recv.collect::<Vec<_>>().await, Vec::new());
        Ok(())
    }

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
        assert_matches!(
            event,
            RuntimeEvent::WorkflowRun {
                event: WorkflowRunEvent::NodeEvents(node_events),
                ..
            } if matches!(node_events[0].status, NodeStatus::Error { ref error, ref detail }
                if error == "Second Context"
                    && *detail == Some("Second Context

Which was caused by:

First Context

Which was caused by:

Root cause".to_string())
        ));

        Ok(())
    }
}
