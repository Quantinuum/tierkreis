/*!
The models module defines the data structures used by the server.
*/
use serde::{Serialize};
use utoipa::ToSchema;
use uuid::Uuid;
use std::sync::Arc;
use tokio::sync::watch;

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
use crate::{
    asset_storage::AssetStorageRegistry,
    state::{
        ConnPool, SqliteRuntimeState,
        interface::RunAttemptUpdated,
    },
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
    pub update_receiver: watch::Receiver<RunAttemptUpdated>,
    pub pool: ConnPool,
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
