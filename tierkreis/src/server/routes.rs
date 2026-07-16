use super::models::{AppState, RuntimeMetadata, WorkflowDisplay};

use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use axum_extra::extract::Query;
use std::collections::HashMap;

use crate::{
    location::Location,
    server::{
        models::{GraphsQuery, GraphsResponse, HandlerResult},
        nodes::{build_py_graph, load_graph, try_load_output_value, try_load_outputs},
    },
    state::{RuntimeState, queries::list_workflow_run_summaries},
};
use miette::IntoDiagnostic;
use uuid::Uuid;

#[utoipa::path(get, path = "/info", responses((status = OK, body = RuntimeMetadata)))]
pub async fn get_info() -> Json<RuntimeMetadata> {
    Json(RuntimeMetadata {
        version: env!("CARGO_PKG_VERSION").to_string(),
    })
}

/// List all workflows in the database, returning a summary of each.
///
/// # Errors
///
/// Returns an internal server error if the database query fails.
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
                .map(std::string::ToString::to_string)
                .collect();
            WorkflowDisplay {
                id: s.run_id, // TODO: THIS IS THE RUN_ID NOT THE WORKFLOW_ID, FIX THIS THIS ALSO AFFECTS `list_nodes()`
                id_int,
                name: s.name,
                start_time: s.started_time.map_or_else(String::new, |t| t.to_rfc3339()),
                errors,
                tkr_version: env!("CARGO_PKG_VERSION").to_string(), // TODO: store the metadata in the database
            }
        })
        .collect();

    Ok(Json(displays))
}

/// Get the graphs for a specific workflow.
///
/// # Errors
///
/// Returns an internal server error if the workflow is not found or if the graph cannot be built.
#[utoipa::path(
    get,
    path = "/workflows/{workflow_id}/graphs",
    params(
        ("workflow_id" = Uuid, Path, description =" The workflow uuid"),
        GraphsQuery,
    ),
    responses(
        (status = OK, body = GraphsResponse),
        (status = 500, description = "Error building graph"),
    )
)]
pub async fn list_nodes(
    State(state): State<AppState>,
    Path(run_id): Path<Uuid>, //TODO currently we only have the RUN_ID from the frontend
    Query(query): Query<GraphsQuery>,
) -> HandlerResult<Json<GraphsResponse>> {
    // Once we get the actual workflow ID the logic needs to be reversed
    // Or the frontend needs to start submitting run_ids
    let run_state = state
        .runtime_state
        .load_workflow_run_state(run_id, 0)
        .await?;
    let workflow_id = run_state.workflow_id();

    tracing::info!("Listing nodes for {}", run_id);

    // TODO can we somehow avoid loading the entire graph (e.g. only the nested ones we need)
    let top_level_graph = state.runtime_state.load_workflow(workflow_id).await?;

    let mut graphs = HashMap::new();
    for loc_str in &query.locs {
        let (graph, prefix) = load_graph(&top_level_graph, loc_str).await?;
        let py_graph =
            build_py_graph(&graph, run_state.as_ref(), &prefix, &state.asset_registry).await?;
        graphs.insert(loc_str.clone(), py_graph);
    }

    Ok(Json(GraphsResponse { graphs }))
}

fn parse_location(s: &str) -> miette::Result<Location> {
    if s.is_empty() || s == "-" {
        Ok(Location::root())
    } else {
        Location::new(s)
    }
}

/// List all outputs for a specific node in a workflow run, returning a map of port name to value.
///
/// # Errors
///
/// Returns an internal server error if the workflow run state cannot be loaded, if the node state cannot be read, or if the outputs cannot be loaded.
#[utoipa::path(
    get,
    path = "/workflows/{workflow_id}/nodes/{node_location_str}/outputs",
    params(
        ("workflow_id" = Uuid, Path, description = "Run ID"),
        ("node_location_str" = String, Path, description = "Location string"),
    ),
    responses(
        (status = OK, description = "JSON object of port name to value"),
        (status = 500, description = "Error loading outputs"),
    )
)]
pub async fn get_all_outputs(
    State(state): State<AppState>,
    Path((run_id, location_str)): Path<(Uuid, String)>,
) -> HandlerResult<Json<HashMap<String, serde_json::Value>>> {
    let run_state = state
        .runtime_state
        .load_workflow_run_state(run_id, 0)
        .await?;

    let loc = parse_location(&location_str)?;
    let node_state = run_state.read(&loc).await?;
    let result = try_load_outputs(&node_state, &state.asset_registry).await?;

    Ok(Json(result))
}

/// List the output for a specific port of a node in a workflow run, returning the raw value as JSON or text.
///
/// # Errors
///
/// Returns an internal server error if the workflow run state cannot be loaded, if the node state cannot be read, or if the output value cannot be loaded.
#[utoipa::path(
    get,
    path = "/workflows/{workflow_id}/nodes/{node_location_str}/outputs/{port_name}",
    params(
        ("workflow_id" = Uuid, Path, description = "Run ID"),
        ("node_location_str" = String, Path, description = "Location string"),
        ("port_name" = String, Path, description = "Output port name"),
    ),
    responses(
        (status = OK, description = "Raw output value as JSON or text"),
        (status = 404, description = "Output not found"),
    )
)]
pub async fn get_single_output(
    State(state): State<AppState>,
    Path((run_id, node_location_str, port_name)): Path<(Uuid, String, String)>,
) -> HandlerResult<Json<serde_json::Value>> {
    let run_state = state
        .runtime_state
        .load_workflow_run_state(run_id, 0)
        .await?;

    let loc = parse_location(&node_location_str)?;
    let node_state = run_state.read(&loc).await?;
    let result = try_load_output_value(&port_name, &node_state, &state.asset_registry).await?;
    Ok(Json(result))
}

/// Get the input for a specific port of a node in a workflow run, returning the raw value as JSON or text.
///
/// # Errors
///
/// Returns an internal server error if the workflow run state cannot be loaded, if the node state cannot be read, or if the input value cannot be loaded.
#[utoipa::path(
    get,
    path = "/workflows/{workflow_id}/nodes/{node_location_str}/inputs/{port_name}",
    params(
        ("workflow_id" = Uuid, Path, description = "Run ID"),
        ("node_location_str" = String, Path, description = "Location string"),
        ("port_name" = String, Path, description = "Output port name"),
    ),
    responses(
        (status = OK, description = "Input value as JSON or text"),
        (status = 404, description = "Input not found"),
    )
)]
pub async fn get_input(
    State(state): State<AppState>,
    Path((run_id, node_location_str, port_name)): Path<(Uuid, String, String)>,
) -> HandlerResult<Response> {
    let run_state = state
        .runtime_state
        .load_workflow_run_state(run_id, 0)
        .await?;

    let workflow_id = run_state.workflow_id();
    let top_level_graph = state.runtime_state.load_workflow(workflow_id).await?;

    let loc = parse_location(&node_location_str)?;
    let node_location = loc.to_string();
    let parent_location = loc.parent();
    let (graph, prefix) = load_graph(&top_level_graph, &parent_location.to_string()).await?;
    // TODO can we avoid constructing this?
    let py_graph =
        build_py_graph(&graph, run_state.as_ref(), &prefix, &state.asset_registry).await?;

    let Some(edge) = py_graph
        .edges
        .into_iter()
        .find(|edge| edge.to_node == node_location && edge.to_port == port_name)
    else {
        return Ok((
            StatusCode::NOT_FOUND,
            format!("Input port '{port_name}' not found for node '{node_location_str}'"),
        )
            .into_response());
    };

    let Some(raw_value) = edge.value else {
        return Ok((
            StatusCode::NOT_FOUND,
            format!("Input for port '{port_name}' is not available yet"),
        )
            .into_response());
    };

    match serde_json::from_str::<serde_json::Value>(&raw_value) {
        Ok(value) => Ok(Json(value).into_response()),
        Err(_) => Ok(raw_value.into_response()),
    }
}
