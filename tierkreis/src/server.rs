/*!
The server module defines the HTTP visualization interface to the Workflow server.
*/
#[allow(missing_docs)]
pub mod models;
#[allow(missing_docs)]
pub mod routes;


use std::sync::Arc;
use axum::{
    http::StatusCode,
};
use miette::IntoDiagnostic;
use utoipa::openapi::OpenApi;
use utoipa_axum::{router::OpenApiRouter, routes};
use utoipa_swagger_ui::SwaggerUi;

use crate::{
    asset_storage::AssetStorageRegistry, 
    state::{
        RuntimeState, SqliteRuntimeState,
        build_conn_pool,
    },
};

/// Server entry point.
pub async fn serve(
    runtime_state: Arc<SqliteRuntimeState>,
    asset_registry: AssetStorageRegistry,
) -> miette::Result<()> {
    let update_receiver = runtime_state
        .listen()
        .map_err(|e| miette::miette!("Failed to register listener: {e:#}"))?;

    let pool = build_conn_pool().await?;

    let app_state = models::AppState {
        runtime_state,
        asset_registry,
        update_receiver,
        pool,
    };

    let api_router = OpenApiRouter::new()
        .routes(routes!(routes::get_info))
        .routes(routes!(routes::list_workflows));
    let (api_http_router, api): (axum::Router<models::AppState>, OpenApi) =
        OpenApiRouter::new().nest("/api", api_router).split_for_parts();
    let mut router = api_http_router.merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", api));

    // SPA
    let dist = std::env::current_dir().unwrap_or_default()
        .join("tierkreis_visualization/tierkreis_visualization/static/dist");
    if dist.exists() {
        let index = dist.join("index.html");
        let assets_dir = dist.join("assets");

        if assets_dir.exists() {
            use tower_http::services::ServeDir;
            router = router.nest_service("/assets", ServeDir::new(&assets_dir));
            tracing::info!("Serving frontend assets from {}", assets_dir.display());
        }
        if index.exists() {
            router = router.fallback(move || {
                let index = index.clone();
                async move {
                    match tokio::fs::read(index).await {
                        Ok(bytes) => axum::response::Response::builder()
                            .header("Content-Type", "text/html; charset=utf-8")
                            .body(axum::body::Body::from(bytes))
                            .unwrap(),
                        Err(_) => axum::response::Response::builder()
                            .status(StatusCode::NOT_FOUND)
                            .body(axum::body::Body::from("index.html not found"))
                            .unwrap(),
                    }
                }
            });
            tracing::info!("Serving frontend SPA from {}", dist.display());
        }
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
