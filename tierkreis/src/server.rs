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
use miette::IntoDiagnostic;
use std::{
    path::{Path, PathBuf},
    sync::Arc,
};
use tower_http::services::ServeFile;
use tower_http::set_status::SetStatus;
use utoipa::openapi::OpenApi;
use utoipa_axum::{router::OpenApiRouter, routes};
use utoipa_swagger_ui::SwaggerUi;

use crate::{
    asset_storage::AssetStorageRegistry,
    runtime::{RuntimeConfig, asset_storage_registry_from_config},
    state::{RuntimeState, SqliteRuntimeState},
};

async fn server(
    runtime_state: Arc<SqliteRuntimeState>,
    asset_registry: AssetStorageRegistry,
    frontend_dist: Option<PathBuf>,
) -> miette::Result<()> {
    let update_receiver = runtime_state.listen();

    let app_state = models::AppState {
        runtime_state,
        asset_registry,
        update_receiver,
    };

    let api_router = OpenApiRouter::new()
        .routes(routes!(routes::get_info))
        .routes(routes!(routes::list_workflows))
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
    if let Some(dist) = frontend_dist {
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
        tracing::info!("No visualization frontend configured; serving the API and Swagger UI only");
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
/// # Errors
///
/// Returns an error if the current directory cannot be read, static frontend files
/// cannot be found, or the HTTP server fails to bind or run.
///
/// # Panics
///
/// Panics if called from an existing Tokio runtime.
pub fn serve() -> miette::Result<()> {
    serve_with_assets(None)
}

/// Server entry point with an optional built visualization frontend directory.
///
/// # Errors
///
/// Returns an error if configuration or frontend assets are invalid, or if the
/// HTTP server cannot start.
///
/// # Panics
///
/// Panics if called from an existing Tokio runtime.
pub fn serve_with_assets(frontend_dist: Option<&Path>) -> miette::Result<()> {
    let config = RuntimeConfig::load()?;
    serve_with_config(&config, frontend_dist)
}

/// Server entry point with explicit runtime configuration and optional frontend.
///
/// # Errors
///
/// Returns an error if configuration or frontend assets are invalid, or if the
/// HTTP server cannot start.
///
/// # Panics
///
/// Panics if called from an existing Tokio runtime.
#[tokio::main]
pub async fn serve_with_config(
    config: &RuntimeConfig,
    frontend_dist: Option<&Path>,
) -> miette::Result<()> {
    config.init_logging();
    let runtime_state = Arc::new(config.sqlite_runtime_state().await?);
    let asset_registry = asset_storage_registry_from_config(config);
    server(
        runtime_state,
        asset_registry,
        frontend_dist.map(Path::to_path_buf),
    )
    .await
}
