/*!
The models module defines the data structures used by the server.
*/
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::watch;
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

use crate::{
    asset_storage::AssetStorageRegistry,
    graph::NodeDefinition,
    state::{
        SqliteRuntimeState,
        interface::{NodeState, RuntimeWatchState},
    },
};
use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};

pub struct AppError(miette::Report);

impl<E: Into<miette::Report>> From<E> for AppError {
    fn from(err: E) -> Self {
        AppError(err.into())
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let body = format!("{:#}", self.0);
        (StatusCode::INTERNAL_SERVER_ERROR, body).into_response()
    }
}

pub type HandlerResult<T> = Result<T, AppError>;

/// Server state shared across all requests.
#[derive(Clone)]
pub struct AppState {
    pub runtime_state: Arc<SqliteRuntimeState>,
    pub asset_registry: AssetStorageRegistry,
    pub update_receiver: watch::Receiver<RuntimeWatchState>,
}

/// Runtime metadata returned by `/api/info`.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct RuntimeMetadata {
    pub version: String,
}

/// Workflow display information returned by `/api/workflows/`.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct WorkflowDisplay {
    /// Currently the run UUID, no attempt/workflow
    pub id: Uuid,
    pub id_int: u64,
    pub name: Option<String>,
    pub start_time: String,
    /// Errored Nodes are taken from Errored time
    pub errors: Vec<String>,
    pub tkr_version: String,
    // TODO: wf id / attempt
}

/// The status of a node in the workflow graph.
/// TODO: Enable remaining states
#[derive(Debug, Clone, Serialize, ToSchema)]
pub enum NodeStatus {
    /// The node has not been reached yet.
    #[serde(rename = "Not started")]
    NotStarted,
    // /// The node has been scheduled by the orchestrator.
    // #[serde(rename = "Scheduled")]
    // Scheduled,
    // /// The node has been queued to run on an executor.
    // #[serde(rename = "Queued")]
    // Queued,
    /// The node is actively running on an executor.
    ///
    /// Maps to "Started" in the Python API for backwards compatibility.
    #[serde(rename = "Started")]
    Running,
    /// The node encountered an error.
    #[serde(rename = "Error")]
    Error,
    /// The node completed successfully.
    #[serde(rename = "Finished")]
    Finished,
    // /// The node was cancelled.
    // #[serde(rename = "Cancelled")]
    // Cancelled,
}

#[must_use]
pub fn node_status_from_state(state: &NodeState) -> NodeStatus {
    if state.complete_time.is_some() {
        NodeStatus::Finished
    } else if state.error_time.is_some() || state.cancelled_time.is_some() {
        NodeStatus::Error
    } else if state.running_time.is_some()
        || state.queued_time.is_some()
        || state.scheduled_time.is_some()
    {
        NodeStatus::Running
    } else {
        NodeStatus::NotStarted
    }
}

impl NodeDefinition {
    #[must_use]
    pub fn node_type(&self) -> String {
        match self {
            NodeDefinition::Input { .. } => "input".to_string(),
            NodeDefinition::Const { .. } => "const".to_string(),
            NodeDefinition::Task { .. } => "function".to_string(),
            NodeDefinition::Map { .. } => "map".to_string(),
            _ => serde_plain::to_string(self).unwrap_or_else(|_| "unknown".to_string()),
        }
    }

    #[must_use]
    pub fn function_name(&self) -> String {
        match self {
            NodeDefinition::Input { name } => name.clone(),
            NodeDefinition::Output {} => "output".to_string(),
            NodeDefinition::Const { .. } => "const".to_string(),
            NodeDefinition::IfElse {} => "ifelse".to_string(),
            NodeDefinition::EagerIfElse {} => "eifelse".to_string(),
            NodeDefinition::Task {
                worker_name,
                task_name,
            } => format!("{worker_name}.{task_name}"),
            NodeDefinition::Eval {} => "eval".to_string(),
            NodeDefinition::Loop {} => "loop".to_string(),
            NodeDefinition::Map { .. } => "map".to_string(),
        }
    }
}
/// Describes a connection into an input port of a node.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct NodeInputs {
    /// The name of the input port on this node.
    pub port: String,
    /// The location of the node providing the value.
    pub from_node: String,
    /// The name of the output port on the source node.
    pub from_port: String,
}

/// A node in the workflow graph, with its current execution status.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct PyNode {
    /// The location of this node as a string.
    pub id: String,
    /// Current execution status.
    pub status: NodeStatus,
    /// Human-readable display name derived from the node definition.
    pub function_name: String,
    /// The structural type of the node.
    pub node_type: String,
    /// Same as `id`; kept for Python API compatibility.
    pub node_location: String,
    /// Names of the output ports.
    pub outputs: Vec<String>,
    /// Incoming connections for each input port.
    pub inputs: Vec<NodeInputs>,
    /// A human-readable value associated with the node (const value, input name, etc.).
    pub value: Option<String>,
    /// ISO-8601 timestamp when the node started running, or `""` if not started.
    pub started_time: String,
    /// ISO-8601 timestamp when the node finished, or `""` if not finished.
    pub finished_time: String,
}

/// A directed edge in the workflow graph.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct PyEdge {
    /// Location of the source node.
    pub from_node: String,
    /// Name of the output port on the source node.
    pub from_port: String,
    /// Location of the target node.
    pub to_node: String,
    /// Name of the input port on the target node.
    pub to_port: String,
    /// Loaded output value at this edge, if available and the node has completed.
    pub value: Option<String>,
    /// Whether this edge is part of a conditional branch.
    pub conditional: bool,
}

/// The graph of nodes and edges at a particular location in the workflow.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct PyGraph {
    /// The nodes in this graph.
    pub nodes: Vec<PyNode>,
    /// The edges connecting nodes in this graph.
    pub edges: Vec<PyEdge>,
}

/// Response containing graphs for multiple requested locations.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct GraphsResponse {
    /// Map from location string to the graph at that location.
    pub graphs: HashMap<String, PyGraph>,
}

/// Query parameters for the `/workflows/{workflow_id}/graphs` endpoint.
#[derive(Debug, Deserialize, IntoParams)]
pub struct GraphsQuery {
    /// Location strings to fetch graphs for (repeatable).
    #[serde(default)]
    pub locs: Vec<String>,
}
