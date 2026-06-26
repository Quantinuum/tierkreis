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
    asset_storage::AssetSpec,
    event::{Event, WorkflowRunEvent},
    graph::WorkflowGraph,
    state::interface::RunAttemptUpdated,
};
use crate::{
    event::{NodeEvent, RunningStateUpdate},
    location::Location,
    state::{
        WorkflowRunState,
        interface::{NodeState, RuntimeState},
    },
};

/// [`RunAttemptState`] is the full state of a run.
#[derive(Debug, Default)]
struct RunAttemptState {
    workflow_id: Uuid,
    inputs: HashMap<String, AssetSpec>,
    nodes: HashMap<Location, NodeState>,
    metadata: HashMap<String, String>,
}

/// [`InMemoryRuntimeStateInner`] is a shared struct that can be accessed
/// by both [`InMemoryRuntimeState`] and [`InMemoryWorkflowRunState`] through shared
/// references.
#[derive(Debug, Default)]
struct InMemoryRuntimeStateInner {
    workflows: DashMap<Uuid, WorkflowGraph>,
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
                workflows: DashMap::new(),
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
    type WorkflowRunState = InMemoryWorkflowRunState;

    async fn load_workflow(&self, workflow_id: Uuid) -> miette::Result<WorkflowGraph> {
        let workflow = self
            .inner
            .workflows
            .get(&workflow_id)
            .ok_or_else(|| miette!("Workflow not found with workflow id: {workflow_id}"))?;
        Ok(workflow.clone())
    }

    async fn save_workflow(&self, workflow_graph: WorkflowGraph) -> miette::Result<Uuid> {
        let workflow_id = Uuid::now_v7();
        self.inner.workflows.insert(workflow_id, workflow_graph);
        Ok(workflow_id)
    }

    async fn new_workflow_run_state(
        &self,
        workflow_id: Uuid,
        inputs: HashMap<String, AssetSpec>,
    ) -> miette::Result<Self::WorkflowRunState> {
        let run_id = Uuid::now_v7();
        let attempt = 0;

        let mut entry = self.inner.runs.entry((run_id, attempt)).or_default();
        entry.workflow_id = workflow_id;
        entry.inputs = inputs;

        Ok(InMemoryWorkflowRunState {
            global_state: Arc::clone(&self.inner),
            update_sender: self.update_sender.clone(),
            workflow_id: entry.workflow_id,
            run_id,
            attempt,
        })
    }

    async fn get_workflow_run_state(
        &self,
        run_id: Uuid,
        attempt: u32,
    ) -> miette::Result<InMemoryWorkflowRunState> {
        let entry = self.inner.runs.entry((run_id, attempt)).or_default();

        Ok(InMemoryWorkflowRunState {
            global_state: Arc::clone(&self.inner),
            update_sender: self.update_sender.clone(),
            workflow_id: entry.workflow_id,
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

/// [`InMemoryWorkflowRunState`] is an implementation of [`WorkflowRunState`] that shares storage
/// with [`InMemoryRuntimeState`].
#[derive(Debug)]
pub struct InMemoryWorkflowRunState {
    global_state: Arc<InMemoryRuntimeStateInner>,
    update_sender: watch::Sender<RunAttemptUpdated>,
    workflow_id: Uuid,
    run_id: Uuid,
    attempt: u32,
}

impl InMemoryWorkflowRunState {
    /// Create a test instance of [`InMemoryWorkflowRunState`] along with a stream of events from a
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
                workflow_id: Uuid::nil(),
                run_id: Uuid::nil(),
                attempt: 0,
            },
            events,
        )
    }
}

impl WorkflowRunState for InMemoryWorkflowRunState {
    fn workflow_id(&self) -> Uuid {
        self.workflow_id
    }

    fn run_id(&self) -> Uuid {
        self.run_id
    }

    fn attempt(&self) -> u32 {
        self.attempt
    }

    async fn load_inputs(&self) -> miette::Result<HashMap<String, AssetSpec>> {
        let run = self
            .global_state
            .runs
            .get(&(self.run_id, self.attempt))
            .ok_or_else(|| miette!("Workflow run not found"))?;
        Ok(run.inputs.clone())
    }

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
        let workflow_run_state = runtime_state
            .get_workflow_run_state(run_id, attempt)
            .await?;

        let node_state = workflow_run_state.read(&Location::root()).await?;

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
        let workflow_run_state = runtime_state
            .get_workflow_run_state(run_id, attempt)
            .await?;

        workflow_run_state
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
        let workflow_run_state = runtime_state
            .get_workflow_run_state(run_id, attempt)
            .await?;

        workflow_run_state
            .write(Event::Node(NodeEvent {
                locs: vec![Location::root()],
                status: NodeStatus::Scheduled,
            }))
            .await?;

        let node_state = workflow_run_state.read(&Location::root()).await?;

        assert!(node_state.scheduled_time.is_some());

        Ok(())
    }

    /// Test that we can read and write metadata
    #[tokio::test]
    async fn write_and_read_metadata() -> miette::Result<()> {
        let runtime_state = InMemoryRuntimeState::new();

        let run_id = Uuid::now_v7();
        let attempt = 0;
        let workflow_run_state = runtime_state
            .get_workflow_run_state(run_id, attempt)
            .await?;

        let metadata = HashMap::from_iter([("foo".to_string(), "bar".to_string())]);
        workflow_run_state.add_metadata(metadata.clone()).await?;

        let read_metadata = workflow_run_state.read_metadata().await?;

        assert_eq!(metadata, read_metadata);

        Ok(())
    }

    /// Test that metadata we write gets merged.
    #[tokio::test]
    async fn merge_metadata() -> miette::Result<()> {
        let runtime_state = InMemoryRuntimeState::new();

        let run_id = Uuid::now_v7();
        let attempt = 0;
        let workflow_run_state = runtime_state
            .get_workflow_run_state(run_id, attempt)
            .await?;

        let mut metadata1 = HashMap::from_iter([("foo".to_string(), "bar".to_string())]);
        workflow_run_state.add_metadata(metadata1.clone()).await?;

        let metadata2 = HashMap::from_iter([("baz".to_string(), "boo".to_string())]);
        workflow_run_state.add_metadata(metadata2.clone()).await?;

        let read_metadata = workflow_run_state.read_metadata().await?;

        metadata1.extend(metadata2.into_iter());
        assert_eq!(metadata1, read_metadata);

        Ok(())
    }
}
