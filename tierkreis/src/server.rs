/*!
The server module defines the REST interface to the Workflow server.
*/
pub mod assets;
#[allow(missing_docs)]
pub mod models;
#[allow(missing_docs)]
pub mod nodes;
#[allow(missing_docs)]
pub mod routes;

use miette::IntoDiagnostic;
use std::net::IpAddr;
use std::sync::Arc;
use utoipa::openapi::OpenApi;
use utoipa_axum::{router::OpenApiRouter, routes};
use utoipa_swagger_ui::SwaggerUi;

use crate::{
    asset_storage::AssetStorageRegistry,
    graph::WorkflowGraph,
    runtime::{RuntimeConfig, asset_storage_registry_from_config},
    state::{InMemoryRuntimeState, RuntimeState, SqliteRuntimeState},
};
use std::net::Ipv4Addr;

fn api_router() -> OpenApiRouter<models::AppState> {
    OpenApiRouter::new()
        .routes(routes!(routes::get_info))
        .routes(routes!(routes::list_workflows))
        .routes(routes!(routes::list_nodes))
        .routes(routes!(routes::get_all_outputs))
        .routes(routes!(routes::get_single_output))
        .routes(routes!(routes::get_input))
        .routes(routes!(routes::get_node_errors))
        .routes(routes!(routes::get_node_logs))
        .routes(routes!(routes::get_workflow_logs))
        .routes(routes!(routes::restart_node))
}

/// Build the `OpenAPI` document for the workflow server.
#[must_use]
pub fn openapi_spec() -> OpenApi {
    let (_, api) = OpenApiRouter::new()
        .nest("/api", api_router())
        .split_for_parts();
    api
}

async fn server(
    runtime_state: Arc<dyn RuntimeState>,
    asset_registry: AssetStorageRegistry,
    host: IpAddr,
    port: u16,
    display_graph: Option<Arc<serde_json::Value>>,
) -> miette::Result<()> {
    let update_receiver = runtime_state.listen();

    let app_state = models::AppState {
        runtime_state,
        asset_registry,
        update_receiver,
        display_graph,
    };

    let (api_http_router, api): (axum::Router<models::AppState>, OpenApi) = OpenApiRouter::new()
        .nest("/api", api_router())
        .split_for_parts();
    let router = api_http_router
        .merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", api))
        .fallback(assets::static_handler)
        .with_state(app_state);

    let listener = tokio::net::TcpListener::bind((host, port))
        .await
        .into_diagnostic()?;
    tracing::info!("Visualization server listening on http://{host}:{port}");
    let (shutdown_sender, shutdown_receiver) = tokio::sync::oneshot::channel::<()>();
    let serving = axum::serve(listener, router).with_graceful_shutdown(async move {
        let _ = shutdown_receiver.await;
    });
    let serving = std::future::IntoFuture::into_future(serving);
    tokio::pin!(serving);
    tokio::select! {
        result = &mut serving => result.into_diagnostic()?,
        signal = tokio::signal::ctrl_c() => {
            signal.into_diagnostic()?;
            tracing::info!("Stopping visualization server");
            let _ = shutdown_sender.send(());
            if let Ok(result) = tokio::time::timeout(
                std::time::Duration::from_secs(5),
                &mut serving,
            ).await {
                result.into_diagnostic()?;
            }
        }
    }

    Ok(())
}

/// Server entry point.
///
/// # Errors
///
/// Returns an error if the runtime state cannot be opened or the HTTP server fails to
/// bind or run.
///
/// # Panics
///
/// Panics if the tokio runtime cannot be started.
#[tokio::main]
pub async fn serve(host: IpAddr, port: u16) -> miette::Result<()> {
    let runtime_state: Arc<dyn RuntimeState> = Arc::new(SqliteRuntimeState::try_new().await?);
    let asset_registry = asset_storage_registry_from_config(&RuntimeConfig::default())?;
    server(runtime_state, asset_registry, host, port, None).await
}

/// Serve a workflow graph in an isolated in-memory runtime state.
///
/// # Errors
///
/// Returns an error if the state cannot be created or the server fails to bind or run.
///
/// # Panics
///
/// Panics if the tokio runtime cannot be started.
#[tokio::main]
pub async fn serve_graph(graph_json: &str, port: u16) -> miette::Result<()> {
    let display_graph: serde_json::Value = serde_json::from_str(graph_json).into_diagnostic()?;
    let runtime_state: Arc<dyn RuntimeState> = Arc::new(InMemoryRuntimeState::new());
    let workflow_id = runtime_state
        .save_workflow(
            Some("Graph".to_string()),
            WorkflowGraph::new(std::iter::empty::<String>()),
        )
        .await?;
    let _run_state = runtime_state
        .new_workflow_run_state(workflow_id, std::collections::HashMap::new())
        .await?;
    let asset_registry = asset_storage_registry_from_config(&RuntimeConfig::default())?;
    server(
        runtime_state,
        asset_registry,
        IpAddr::V4(Ipv4Addr::LOCALHOST),
        port,
        Some(Arc::new(display_graph)),
    )
    .await
}
