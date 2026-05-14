/*!
This module defines the [`InMemoryRuntimeState`] struct which implements [`RuntimeState`]
that can be used by the tierkreis runtime.

These implementations are intended to be used for testing and debugging as their
state is not persisted beyond the lifetime of the process.
*/
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use chrono::Utc;
use dashmap::DashMap;
use futures::{
    FutureExt, SinkExt, StreamExt,
    channel::mpsc,
    future::{self, BoxFuture},
    stream::BoxStream,
};
use miette::miette;
use uuid::Uuid;

use crate::state::interface::RunAttemptUpdated;
use crate::{
    event::Event,
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
    update_sender: mpsc::Sender<RunAttemptUpdated>,
    update_receiver: Mutex<Option<mpsc::Receiver<RunAttemptUpdated>>>,
}

impl InMemoryRuntimeState {
    /// Create a new [`InMemoryRuntimeState`] instance.
    #[must_use]
    pub fn new() -> Self {
        let (sender, receiver) = mpsc::channel(128);
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
    fn workflow_state(&self, run_id: Uuid, attempt: u32) -> Arc<dyn WorkflowState> {
        let entry = self.inner.runs.entry((run_id, attempt));
        // Insert the default if the value does not yet exist.
        entry.or_default();

        Arc::new(InMemoryWorkflowState {
            global_state: Arc::clone(&self.inner),
            update_sender: self.update_sender.clone(),
            run_id,
            attempt,
        })
    }

    fn listen(&self) -> miette::Result<BoxStream<'static, RunAttemptUpdated>> {
        let receiver = {
            let mut receiver = self
                .update_receiver
                .try_lock()
                .map_err(|err| miette!("Failed to listen: {}", err))?;

            receiver.take().ok_or(miette!(
                "Failed to listen: InMemoryGlobalState is already being listened to."
            ))?
        };

        Ok(receiver.boxed())
    }
}

/// [`InMemoryWorkflowState`] is an implementation of [`WorkflowState`] that shares storage
/// with [`InMemoryRuntimeState`].
#[derive(Debug)]
pub struct InMemoryWorkflowState {
    global_state: Arc<InMemoryRuntimeStateInner>,
    update_sender: mpsc::Sender<RunAttemptUpdated>,
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
    pub fn test() -> (Self, BoxStream<'static, RunAttemptUpdated>) {
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
    fn write(&self, event: Event) -> BoxFuture<'_, miette::Result<()>> {
        let global_state = &self.global_state;
        let mut run_state = global_state
            .runs
            .entry((self.run_id, self.attempt))
            .or_default();

        let mut send_update = false;
        let mut send_workflow_complete = false;
        let node_state = run_state.nodes.entry(event.loc).or_default();

        match event.status {
            crate::event::Status::Scheduled => {
                if node_state.scheduled_time.is_none() {
                    send_update = true;
                    node_state.scheduled_time = Some(Utc::now());
                }
            }
            crate::event::Status::Queued => {
                if node_state.queued_time.is_none() {
                    send_update = true;
                    node_state.queued_time = Some(Utc::now());
                }
            }
            crate::event::Status::Running => {
                if node_state.running_time.is_none() {
                    send_update = true;
                    node_state.running_time = Some(Utc::now());
                }
            }
            crate::event::Status::Complete {
                outputs,
                workflow_complete,
            } => {
                if node_state.complete_time.is_none() {
                    send_update = true;
                    node_state.complete_time = Some(Utc::now());
                    node_state.outputs = Some(outputs);

                    send_workflow_complete = workflow_complete;
                }
            }
            crate::event::Status::Cancelled => {
                if node_state.cancelled_time.is_none() {
                    send_update = true;
                    node_state.cancelled_time = Some(Utc::now());
                }
            }
            crate::event::Status::Error { error, detail } => {
                if node_state.error_time.is_none() {
                    send_update = true;
                    node_state.error_time = Some(Utc::now());
                    node_state.error = Some(error);
                    node_state.error_detail = detail;
                }
            }
        }

        let mut update_sender = self.update_sender.clone();
        async move {
            if send_update {
                update_sender
                    .send(RunAttemptUpdated {
                        run_id: self.run_id,
                        attempt: self.attempt,
                        complete: send_workflow_complete,
                    })
                    .await
                    .map_err(|err| miette!("Send failed: {err}"))?;
            }
            Ok(())
        }
        .boxed()
    }

    fn read(&self, location: &Location) -> BoxFuture<'_, miette::Result<NodeState>> {
        let global_state = &self.global_state;
        let res = || {
            let run_state = global_state
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
        };

        future::ready(res()).boxed()
    }
}

#[cfg(test)]
mod tests {
    use crate::event::Status;

    use super::*;

    #[tokio::test]
    async fn listen_for_updates() -> miette::Result<()> {
        let runtime_state = InMemoryRuntimeState::new();

        let stream = runtime_state.listen().expect("Failed to listen");

        let run_id = Uuid::now_v7();
        let attempt = 0;
        let workflow_state = runtime_state.workflow_state(run_id, attempt);

        workflow_state
            .write(Event {
                loc: Location::root(),
                status: Status::Scheduled,
            })
            .await
            .expect("Failed to write state");

        let updated = stream.take(1).collect::<Vec<_>>().await;
        assert_eq!(updated.len(), 1);
        assert_eq!(
            updated[0],
            RunAttemptUpdated {
                run_id,
                attempt,
                complete: false
            }
        );

        Ok(())
    }
}
