use axum::{Json, extract::Path};
use chrono::Utc;
use miette::IntoDiagnostic;
use utoipa::openapi::OpenApi;
use utoipa_axum::{router::OpenApiRouter, routes};
use utoipa_swagger_ui::SwaggerUi;
use uuid::Uuid;

#[derive(utoipa::ToSchema, serde::Serialize)]
struct Workflow {
    id: Uuid,
    name: String,
}

#[utoipa::path(get, path = "/workflows", responses((status = OK, body = Vec<Workflow>)))]
async fn get_workflows() -> Json<Vec<Workflow>> {
    Json(vec![])
}

#[derive(utoipa::ToSchema, serde::Serialize)]
struct WorkflowError {
    detail: String,
}

#[derive(utoipa::ToSchema, serde::Serialize)]
struct WorkflowRun {
    id: Uuid,
    start_time: chrono::DateTime<Utc>,
    errors: Vec<WorkflowError>,
}

#[utoipa::path(get, path = "/workflows/{workflow_id}/runs", responses((status = OK, body = Vec<WorkflowRun>)))]
async fn get_workflow_runs(workflow_id: Path<Uuid>) -> Json<Vec<WorkflowRun>> {
    let _workflow_id = workflow_id;
    Json(vec![])
}

#[tokio::main]
pub async fn serve() -> miette::Result<()> {
    let api_router = OpenApiRouter::new()
        .routes(routes!(get_workflows))
        .routes(routes!(get_workflow_runs));

    let (router, api): (axum::Router, OpenApi) = OpenApiRouter::new()
        .nest("/api", api_router)
        .split_for_parts();

    let router = router.merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", api));
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000")
        .await
        .into_diagnostic()?;
    axum::serve(listener, router).await.into_diagnostic()?;
    Ok(())
}
