/*!
The server module defines the REST interface to the Workflow server.
*/
#[allow(missing_docs)]
pub mod models;
#[allow(missing_docs)]
pub mod nodes;
#[allow(missing_docs)]
pub mod routes;

use axum::http::StatusCode;
use miette::{IntoDiagnostic, WrapErr};
use std::sync::Arc;
use tower_http::services::ServeFile;
use tower_http::set_status::SetStatus;
use utoipa::openapi::OpenApi;
use utoipa_axum::{router::OpenApiRouter, routes};
use utoipa_swagger_ui::SwaggerUi;

use crate::runtime::{Runtime, RuntimeConfig};

async fn server(runtime: Arc<Runtime>) -> miette::Result<()> {
    let update_receiver = runtime.listen();

    let app_state = models::AppState {
        runtime_state: runtime.state(),
        asset_registry: runtime.asset_storage_registry(),
        runtime,
        update_receiver,
    };

    let api_router = OpenApiRouter::new()
        .routes(routes!(routes::get_info))
        .routes(routes!(routes::list_workflows))
        .routes(routes!(routes::get_monitoring_summary))
        .routes(routes!(routes::get_workflows_summary))
        .routes(routes!(routes::get_runtime_info))
        .routes(routes!(routes::get_run_trace))
        .routes(routes!(routes::start_new_run))
        .routes(routes!(routes::get_workflow_input_names))
        .routes(routes!(routes::start_new_attempt))
        .routes(routes!(routes::list_nodes))
        .routes(routes!(routes::get_all_outputs))
        .routes(routes!(routes::get_single_output))
        .routes(routes!(routes::get_input))
        .routes(routes!(routes::get_node_errors))
        .routes(routes!(routes::get_node_logs))
        .routes(routes!(routes::get_workflow_logs));
    let (api_http_router, api): (axum::Router<models::AppState>, OpenApi) = OpenApiRouter::new()
        .nest("/api", api_router)
        .split_for_parts();
    let mut router =
        api_http_router.merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", api));

    // SPA
    let dist = std::env::current_dir()
        .into_diagnostic()
        .wrap_err("Failed to resolve current working directory")?
        .join("tierkreis_visualization/tierkreis_visualization/static/dist"); // TODO: Make this configurable
    if dist.exists() {
        let index = dist.join("index.html");
        let assets_dir = dist.join("assets");
        if !index.exists() {
            return Err(miette::miette!(
                "Static frontend index.html not found: {}",
                index.display()
            ));
        }

        if assets_dir.exists() {
            use tower_http::services::ServeDir;
            let index_html = SetStatus::new(ServeFile::new(&index), StatusCode::NOT_FOUND);
            router = router
                .nest_service("/assets", ServeDir::new(&assets_dir))
                .fallback_service(index_html);
            tracing::info!("Serving frontend assets from {}", assets_dir.display());
        }
        tracing::info!("Serving frontend SPA from {}", dist.display());
    } else {
        return Err(miette::miette!(
            "Static frontend directory not found: {}",
            dist.display()
        ));
    }

    let router = router.with_state(app_state);
    let port = 3000;
    let listener = tokio::net::TcpListener::bind(("0.0.0.0", port))
        .await
        .into_diagnostic()?;
    tracing::info!("Visualization server listening on http://localhost:{port}");
    axum::serve(listener, router).await.into_diagnostic()?;

    Ok(())
}

/// Server entry point.
///
/// Builds a [`Runtime`] backed by persistent `SQLite` state and runs its
/// orchestration loop alongside the HTTP API in this same process, so that
/// runs/attempts started via the API are actually executed.
///
/// # Errors
///
/// Returns an error if the current directory cannot be read, static frontend files
/// cannot be found, or the HTTP server fails to bind or run.
///
/// # Panics
///
/// Panics if the static frontend files are not found in the expected location.
#[tokio::main]
pub async fn serve() -> miette::Result<()> {
    let runtime = Arc::new(Runtime::from_config(&RuntimeConfig::persistent()).await?);
    let orchestration_runtime = Arc::clone(&runtime);
    let orchestration = async move { orchestration_runtime.run().await };
    // Polled concurrently on this same task (no `tokio::spawn`), since the
    // orchestration loop's internal streams are not `Send`.
    tokio::try_join!(server(runtime), orchestration)?;
    Ok(())
}
