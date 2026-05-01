/*!
This module defines the interface contracts that the various [`RuntimeState`]
and [`WorkflowState`] implementations must satisfy.
*/
use std::{collections::HashMap, fmt::Debug, sync::Arc};

use chrono::{DateTime, Utc};
use futures::{FutureExt, future::BoxFuture, stream::BoxStream};
use uuid::Uuid;

use crate::{asset_storage::AssetSpec, event::Event, location::Location};

/// [`RunAttemptUpdated`] is a struct that is emitted by the [`RuntimeState`] interface
/// whenever a run attempt changes, in order to drive further workflow orchestration.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct RunAttemptUpdated {
    /// The unique identifier of the run.
    pub run_id: Uuid,
    /// The number of the run attempt, typically sequential.
    pub attempt: u32,
    /// Whether the workflow is complete/cancelled/errored and no further orchestration is required.
    pub stopped: bool,
}

/// [`NodeState`] is a struct that stores the possible state that a node
/// in the Workflow graph can be in.
///
/// This state is built up by reading [`Event`] messages and can be queried
/// by the [`WorkflowState`] interface.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct NodeState {
    /// The time at which the node was scheduled by the [`Orchestrator`] if any.
    pub scheduled_time: Option<DateTime<Utc>>,
    /// The time at which the node was queued by an [`Executor`] if any.
    pub queued_time: Option<DateTime<Utc>>,
    /// The time at which the node started running by an [`Executor`] if any.
    pub running_time: Option<DateTime<Utc>>,
    /// The time at which the node was completed by the [`Orchestrator`] or an [`Executor`] if any.
    pub complete_time: Option<DateTime<Utc>>,
    /// The time at which the node was cancelled by the [`Executor`] if any.
    pub cancelled_time: Option<DateTime<Utc>>,
    /// The time at which the node errored as reported by the [`Orchestrator`] or an [`Executor`] if any.
    pub error_time: Option<DateTime<Utc>>,

    /// The outputs of the node and their stored locations if any.
    pub outputs: Option<HashMap<String, AssetSpec>>,

    /// The error message of the node if any.
    pub error: Option<String>,
    /// The detail of the error for the node if any.
    pub error_detail: Option<String>,
}

/// [`RuntimeState`] is an interface to the state of the overall tierkreis runtime, across
/// all of the running and completed Workflows.
pub trait RuntimeState: Debug + Send + Sync {
    /// Retrieve a handle to a [`WorkflowState`] depending on the `run_id` and attempt number.
    ///
    /// If the backing data for the [`WorkflowState`] does not exist, create it.
    ///
    /// The [`WorkflowState`] comes inside an `Arc` such that it can be shared between threads.
    fn workflow_state(&self, run_id: Uuid, attempt: u32) -> Arc<dyn WorkflowState>;
    /// Listen for updates about *all* of the running workflows.
    ///
    /// # Errors
    ///
    /// Will return Err if the method has already been called.
    fn listen(&self) -> miette::Result<BoxStream<'static, RunAttemptUpdated>>;
}

/// [`WorkflowState`] is an interface to the state of an individual Workflow run attempt.
pub trait WorkflowState: Debug + Send + Sync {
    /// Update the [`WorkflowState`] from an [`Event`].
    fn write(&self, event: Event) -> BoxFuture<'_, miette::Result<()>>;
    /// Read the state of a Node at the specified [`Location`].
    ///
    /// If the `location` has no existing state, a default [`NodeState`] will be returned.
    fn read(&self, location: &Location) -> BoxFuture<'_, miette::Result<NodeState>>;
    /// Add metadata for the Workflow run. The new metadata will be merged with the existing values.
    fn add_metadata(&self, metadata: HashMap<String, String>) -> BoxFuture<'_, miette::Result<()>>;
    /// Read the metadata for the Workflow run.
    fn read_metadata(&self) -> BoxFuture<'_, miette::Result<HashMap<String, String>>>;

    /// Returns `Ok(true)` if the Node at the specified [`Location`] has been
    /// scheduled by the [`Orchestrator`].
    fn is_scheduled(&self, location: &Location) -> BoxFuture<'_, miette::Result<bool>> {
        self.read(location)
            .map(|res| res.map(|state| state.scheduled_time.is_some()))
            .boxed()
    }

    /// Returns `Ok(true)` if the Node at the specified [`Location`] has recorded
    /// outputs, which suggests that it has completed.
    fn has_outputs(&self, location: &Location) -> BoxFuture<'_, miette::Result<bool>> {
        self.read(location)
            .map(|res| res.map(|state| state.outputs.is_some()))
            .boxed()
    }
}
