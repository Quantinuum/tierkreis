use super::models::{WorkflowDisplay, RuntimeMetadata, AppState};

use axum::{
    Json,
    extract::State,
};
use miette::IntoDiagnostic;

use crate::{
    server::models::HandlerResult, state::queries::list_workflow_run_summaries,
};


#[utoipa::path(get, path = "/info", responses((status = OK, body = RuntimeMetadata)))]
pub async fn get_info() -> Json<RuntimeMetadata> {
    Json(RuntimeMetadata {
        version: env!("CARGO_PKG_VERSION").to_string(),
    })
}

#[utoipa::path(
    get,
    path = "/workflows/",
    responses((status = OK, body = Vec<WorkflowDisplay>))
)]
pub async fn list_workflows(
    State(state): State<AppState>,
) -> HandlerResult<Json<Vec<WorkflowDisplay>>> {
    let mut conn = state.pool.get().await.into_diagnostic()?;
    let summaries = list_workflow_run_summaries(&mut conn).await?;

    let displays: Vec<WorkflowDisplay> = summaries
        .into_iter()
        .map(|s| {
            let id_int = {
                let bytes = s.run_id.as_bytes();
                u64::from_be_bytes(bytes[0..8].try_into().unwrap_or([0u8; 8]))
            };
            let errors: Vec<String> = s
                .errored_locations
                .iter()
                .map(|loc| loc.to_string())
                .collect();
            WorkflowDisplay {
                id: s.run_id,
                id_int,
                name: s.name,
                start_time: s
                    .started_time
                    .map_or_else(String::new, |t| t.to_rfc3339()),
                errors,
                tkr_version: env!("CARGO_PKG_VERSION").to_string(),
            }
        })
        .collect();

    Ok(Json(displays))
}
