use super::models::{AppState, RuntimeMetadata, WorkflowDisplay};

use axum::{
    Json,
    extract::{Path, State},
};
use axum_extra::extract::Query;
use std::collections::HashMap;

use crate::{
    location::Location,
    server::{
        models::{GraphsQuery, GraphsResponse, HandlerResult},
        nodes::{build_py_graph, load_graph, try_load_output_value, try_load_outputs},
    },
    state::{RuntimeState, WorkflowRunState, queries::list_workflow_run_summaries},
};
use miette::IntoDiagnostic;
use uuid::Uuid;

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

    //let loc = Location::root();
    // TODO can we somehow avoid loading the entire graph (e.g. only the nested ones we need)
    let top_level_graph = state.runtime_state.load_workflow(workflow_id).await?;
    //let py_graph = build_py_graph(&top_level_graph, &run_state, &loc, &state.asset_registry).await?;

    let mut graphs = HashMap::new();
    //graphs.insert(loc.to_string(), py_graph);
    for loc_str in &query.locs {
        tracing::info!("Loading graph with prefix {}", loc_str);
        let (graph, prefix) = load_graph(&top_level_graph, loc_str).await?;
        tracing::info!("Loaded {:?} with prefix {}", graph, prefix.to_string());
        let py_graph = build_py_graph(&graph, &run_state, &prefix, &state.asset_registry).await?;
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
