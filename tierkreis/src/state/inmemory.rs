/*!
This module defines the [`InMemoryRuntimeState`] struct which implements [`RuntimeState`]
that can be used by the tierkreis runtime.

These implementations are intended to be used for testing and debugging as their
state is not persisted beyond the lifetime of the process.
*/
use std::{
    collections::HashMap,
    ops::BitOrAssign,
    sync::{Arc, Mutex},
};

use bitvec::vec::BitVec;
use chrono::Utc;
use dashmap::DashMap;
use miette::miette;
use tokio::sync::watch;
use tracing::instrument;
use uuid::Uuid;

use crate::{
    event::{Event, WorkflowRunEvent},
    state::interface::RunAttemptUpdated,
};
use crate::{
    event::{NodeEvent, RunningStateUpdate},
    location::Location,
    state::{
        WorkflowState,
        interface::{NodeState, RuntimeState},
    },
};

/// [`RunAttemptState`] is the full state of a run.
#[derive(Debug, Default)]
struct RunAttemptState {
    nodes: HashMap<Location, NodeState>,
    metadata: HashMap<String, String>,
}

/// [`InMemoryRuntimeStateInner`] is a shared struct that can be accessed
/// by both [`InMemoryRuntimeState`] and [`InMemoryWorkflowState`] through shared
/// references.
#[derive(Debug, Default)]
struct InMemoryRuntimeStateInner {
    runs: DashMap<(Uuid, u32), RunAttemptState>,
}

/// [`InMemoryRuntimeState`] implements [`RuntimeState`] but with an in-memory backing
/// that will not be persisted outside of the tierkreis runtime process.
#[derive(Debug)]
pub struct InMemoryRuntimeState {
    inner: Arc<InMemoryRuntimeStateInner>,
    update_sender: watch::Sender<RunAttemptUpdated>,
    update_receiver: Mutex<Option<watch::Receiver<RunAttemptUpdated>>>,
}

impl InMemoryRuntimeState {
    /// Create a new [`InMemoryRuntimeState`] instance.
    #[must_use]
    pub fn new() -> Self {
        let (sender, receiver) = watch::channel(RunAttemptUpdated {
            attempt: 0,
            run_id: Uuid::nil(),
            stopped: false,
        });
        Self {
            inner: Arc::new(InMemoryRuntimeStateInner {
                runs: DashMap::new(),
            }),

            update_sender: sender,
            update_receiver: Mutex::new(Some(receiver)),
        }
    }
}

impl Default for InMemoryRuntimeState {
    fn default() -> Self {
        Self::new()
    }
}

impl RuntimeState for InMemoryRuntimeState {
    type WorkflowState = InMemoryWorkflowState;

    async fn workflow_state(
        &self,
        run_id: Uuid,
        attempt: u32,
    ) -> miette::Result<InMemoryWorkflowState> {
        let entry = self.inner.runs.entry((run_id, attempt));
        // Insert the default if the value does not yet exist.
        entry.or_default();

        Ok(InMemoryWorkflowState {
            global_state: Arc::clone(&self.inner),
            update_sender: self.update_sender.clone(),
            run_id,
            attempt,
        })
    }

    fn listen(&self) -> miette::Result<watch::Receiver<RunAttemptUpdated>> {
        let receiver = {
            let mut receiver = self
                .update_receiver
                .try_lock()
                .map_err(|err| miette!("Failed to listen: {}", err))?;

            receiver.take().ok_or(miette!(
                "Failed to listen: InMemoryRuntimeState is already being listened to."
            ))?
        };

        Ok(receiver)
    }
}

/// [`InMemoryWorkflowState`] is an implementation of [`WorkflowState`] that shares storage
/// with [`InMemoryRuntimeState`].
#[derive(Debug)]
pub struct InMemoryWorkflowState {
    global_state: Arc<InMemoryRuntimeStateInner>,
    update_sender: watch::Sender<RunAttemptUpdated>,
    run_id: Uuid,
    attempt: u32,
}

impl InMemoryWorkflowState {
    /// Create a test instance of [`InMemoryWorkflowState`] along with a stream of events from a
    /// paired [`InMemoryRuntimeState`].
    ///
    /// # Panics
    ///
    /// Will panic if `listen` returns an error, but this should be impossible.
    #[cfg(test)]
    #[must_use]
    pub fn test() -> (Self, watch::Receiver<RunAttemptUpdated>) {
        let global_state = InMemoryRuntimeState::new();
        let events = global_state.listen().unwrap();
        global_state
            .inner
            .runs
            .insert((Uuid::nil(), 0), RunAttemptState::default());
        (
            Self {
                global_state: Arc::clone(&global_state.inner),
                update_sender: global_state.update_sender.clone(),
                run_id: Uuid::nil(),
                attempt: 0,
            },
            events,
        )
    }
}

impl WorkflowState for InMemoryWorkflowState {
    #[instrument]
    async fn write(&self, event: Event) -> miette::Result<()> {
        let global_state = &self.global_state;
        let run_state = global_state
            .runs
            .entry((self.run_id, self.attempt))
            .or_default();

        let mut send_workflow_stopped = false;

        match event {
            Event::WorkflowRun(ref run_event) => {
                handle_run_event(&mut send_workflow_stopped, run_event);
            }
            Event::Node(ref node_event) => handle_node_event(run_state, node_event),
        }

        self.update_sender.send_modify(|run_attempt_updated| {
            run_attempt_updated.run_id = self.run_id;
            run_attempt_updated.attempt = self.attempt;
            run_attempt_updated.stopped |= send_workflow_stopped;
        });
        Ok(())
    }

    #[instrument]
    async fn read(&self, location: &Location) -> miette::Result<NodeState> {
        let run_state = self
            .global_state
            .runs
            .get(&(self.run_id, self.attempt))
            .ok_or_else(|| {
                miette!(
                    "Run Attempt with id {} and attempt {} not found",
                    self.run_id,
                    self.attempt
                )
            })?;
        let state = run_state
            .value()
            .nodes
            .get(location)
            .cloned()
            .unwrap_or_default();

        Ok(state.clone())
    }

    async fn add_metadata(&self, metadata: HashMap<String, String>) -> miette::Result<()> {
        let entry = self.global_state.runs.entry((self.run_id, self.attempt));
        entry.or_default().metadata.extend(metadata);
        Ok(())
    }

    async fn read_metadata(&self) -> miette::Result<HashMap<String, String>> {
        let entry = self.global_state.runs.entry((self.run_id, self.attempt));
        let metadata = entry.or_default().value().metadata.clone();
        Ok(metadata)
    }
}

fn handle_run_event(send_workflow_stopped: &mut bool, run_event: &WorkflowRunEvent) {
    match run_event {
        WorkflowRunEvent::Started {} => {}
        WorkflowRunEvent::Cancelled {}
        | WorkflowRunEvent::Errored {}
        | WorkflowRunEvent::Completed {} => *send_workflow_stopped = true,
    }
}

fn handle_node_event(
    mut run_state: dashmap::mapref::one::RefMut<'_, (Uuid, u32), RunAttemptState>,
    node_event: &NodeEvent,
) {
    let now = Utc::now();
    for (idx, loc) in node_event.locs.iter().enumerate() {
        let node_state = run_state.nodes.entry(loc.clone()).or_default();
        match node_event.status {
            crate::event::NodeStatus::Scheduled => {
                if node_state.scheduled_time.is_none() {
                    node_state.scheduled_time = Some(now);
                }
            }
            crate::event::NodeStatus::Queued => {
                if node_state.queued_time.is_none() {
                    node_state.queued_time = Some(now);
                }
            }
            crate::event::NodeStatus::Running { state_update: None } => {
                if node_state.running_time.is_none() {
                    node_state.running_time = Some(now);
                }
            }
            crate::event::NodeStatus::Running {
                state_update: Some(RunningStateUpdate::Switching { cond }),
            } => {
                if node_state.running_time.is_none() {
                    node_state.running_time = Some(now);
                }
                if node_state.cond.is_none() {
                    node_state.cond = Some(cond);
                }
            }
            crate::event::NodeStatus::Running {
                state_update: Some(RunningStateUpdate::Looping { index }),
            } => {
                if node_state.running_time.is_none() {
                    node_state.running_time = Some(now);
                }
                if node_state.loop_index != Some(index) {
                    node_state.loop_index = Some(index);
                }
            }
            crate::event::NodeStatus::Running {
                state_update: Some(RunningStateUpdate::MapStarted { size }),
            } => {
                if node_state.running_time.is_none() {
                    node_state.running_time = Some(now);
                }
                if node_state.map_completed.is_none() {
                    node_state.map_completed = Some(BitVec::repeat(false, size as usize));
                }
            }
            crate::event::NodeStatus::Running {
                state_update: Some(RunningStateUpdate::MapElemComplete { ref bits }),
            } => {
                if node_state.running_time.is_none() {
                    node_state.running_time = Some(now);
                }
                if let Some(map_completed) = node_state.map_completed.as_mut() {
                    map_completed.bitor_assign(bits);
                }
            }
            crate::event::NodeStatus::Complete { ref outputs } => {
                if node_state.complete_time.is_none() {
                    node_state.complete_time = Some(now);
                    node_state.outputs = Some(outputs.get(idx).unwrap().clone());
                }
            }
            crate::event::NodeStatus::Cancelled => {
                if node_state.cancelled_time.is_none() {
                    node_state.cancelled_time = Some(now);
                }
            }
            crate::event::NodeStatus::Error {
                ref error,
                ref detail,
            } => {
                if node_state.error_time.is_none() {
                    node_state.error_time = Some(now);
                    node_state.error = Some(error.clone());
                    node_state.error_detail.clone_from(detail);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::event::NodeStatus;

    use super::*;

    /// Test that reading a location returns the default value.
    #[tokio::test]
    async fn read_location_returns_default() -> miette::Result<()> {
        let runtime_state = InMemoryRuntimeState::new();

        let run_id = Uuid::now_v7();
        let attempt = 0;
        let workflow_state = runtime_state.workflow_state(run_id, attempt).await?;

        let node_state = workflow_state.read(&Location::root()).await?;

        assert_eq!(node_state, NodeState::default());

        Ok(())
    }

    /// Test that we can write and listen for updates.
    #[tokio::test]
    async fn write_and_listen_for_updates() -> miette::Result<()> {
        let runtime_state = InMemoryRuntimeState::new();

        let mut recv = runtime_state.listen()?;

        let run_id = Uuid::now_v7();
        let attempt = 0;
        let workflow_state = runtime_state.workflow_state(run_id, attempt).await?;

        workflow_state
            .write(Event::Node(NodeEvent {
                locs: vec![Location::root()],
                status: NodeStatus::Scheduled,
            }))
            .await?;

        let updated = recv.borrow_and_update();
        assert_eq!(
            *updated,
            RunAttemptUpdated {
                run_id,
                attempt,
                stopped: false,
            }
        );

        Ok(())
    }

    /// Test that we can read and write workflow run state.
    #[tokio::test]
    async fn write_and_read() -> miette::Result<()> {
        let runtime_state = InMemoryRuntimeState::new();

        let run_id = Uuid::now_v7();
        let attempt = 0;
        let workflow_state = runtime_state.workflow_state(run_id, attempt).await?;

        workflow_state
            .write(Event::Node(NodeEvent {
                locs: vec![Location::root()],
                status: NodeStatus::Scheduled,
            }))
            .await?;

        let node_state = workflow_state.read(&Location::root()).await?;

        assert!(node_state.scheduled_time.is_some());

        Ok(())
    }

    /// Test that we can read and write metadata
    #[tokio::test]
    async fn write_and_read_metadata() -> miette::Result<()> {
        let runtime_state = InMemoryRuntimeState::new();

        let run_id = Uuid::now_v7();
        let attempt = 0;
        let workflow_state = runtime_state.workflow_state(run_id, attempt).await?;

        let metadata = HashMap::from_iter([("foo".to_string(), "bar".to_string())]);
        workflow_state.add_metadata(metadata.clone()).await?;

        let read_metadata = workflow_state.read_metadata().await?;

        assert_eq!(metadata, read_metadata);

        Ok(())
    }

    /// Test that metadata we write gets merged.
    #[tokio::test]
    async fn merge_metadata() -> miette::Result<()> {
        let runtime_state = InMemoryRuntimeState::new();

        let run_id = Uuid::now_v7();
        let attempt = 0;
        let workflow_state = runtime_state.workflow_state(run_id, attempt).await?;

        let mut metadata1 = HashMap::from_iter([("foo".to_string(), "bar".to_string())]);
        workflow_state.add_metadata(metadata1.clone()).await?;

        let metadata2 = HashMap::from_iter([("baz".to_string(), "boo".to_string())]);
        workflow_state.add_metadata(metadata2.clone()).await?;

        let read_metadata = workflow_state.read_metadata().await?;

        metadata1.extend(metadata2.into_iter());
        assert_eq!(metadata1, read_metadata);

        Ok(())
    }
}
