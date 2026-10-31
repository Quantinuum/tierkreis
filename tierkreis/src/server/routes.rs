use super::models::{AppState, RuntimeMetadata, WorkflowDisplay};

use axum::{
    Json,
    extract::{
        Path, State, WebSocketUpgrade,
        ws::{CloseFrame, Message, WebSocket},
    },
    http::StatusCode,
    response::{IntoResponse, Response},
};
use axum_extra::extract::Query;
use std::{collections::HashMap, sync::Arc, time::Duration};
use tokio::{
    sync::watch,
    time::{MissedTickBehavior, interval},
};

use crate::{
    location::Location,
    server::{
        models::{GraphsQuery, GraphsResponse, HandlerResult},
        nodes::{
            GraphLoadResult, build_loop_py_graph, build_map_py_graph, build_py_graph, load_graph,
            try_load_output_value, try_load_outputs,
        },
    },
    state::{RuntimeWatchState, WorkflowRunState},
};
use uuid::Uuid;

#[utoipa::path(get, path = "/info", responses((status = OK, body = RuntimeMetadata)))]
pub async fn get_info() -> Json<RuntimeMetadata> {
    Json(RuntimeMetadata {
        version: env!("CARGO_PKG_VERSION").to_string(),
    })
}

/// Notify the frontend to refresh graph state, including changes from another process.
///
/// # Errors
///
/// Returns an error if the node location string cannot be parsed or the workflow run is unknown.
#[utoipa::path(
    get,
    path = "/workflows/{workflow_id}/nodes/{node_location_str}",
    params(
        ("workflow_id" = Uuid, Path, description = "Run ID"),
        ("node_location_str" = String, Path, description = "Location string"),
    ),
    responses((status = 101, description = "Graph refresh notifications"))
)]
pub async fn subscribe_node_updates(
    State(state): State<AppState>,
    Path((run_id, location_str)): Path<(Uuid, String)>,
    websocket: WebSocketUpgrade,
) -> HandlerResult<Response> {
    parse_location(&location_str)?;
    let run_state = if state.display_graph.is_some() {
        None
    } else {
        if !state
            .runtime_state
            .list_workflow_run_summaries()
            .await?
            .iter()
            .any(|summary| summary.run_id == run_id)
        {
            return Err(miette::miette!("Unknown workflow run {run_id}").into());
        }
        Some(
            state
                .runtime_state
                .load_workflow_run_state(run_id, 0)
                .await?,
        )
    };
    let mut update_receiver = state.update_receiver;
    update_receiver.mark_unchanged();
    Ok(websocket.on_upgrade(move |socket| stream_node_updates(socket, run_state, update_receiver)))
}

async fn stream_node_updates(
    mut socket: WebSocket,
    run_state: Option<Arc<dyn WorkflowRunState>>,
    mut update_receiver: watch::Receiver<RuntimeWatchState>,
) {
    // The fallback detects writes made through a different runtime state instance or process.
    let mut updates = interval(Duration::from_secs(1));
    updates.set_missed_tick_behavior(MissedTickBehavior::Skip);
    let mut watching = true;
    loop {
        tokio::select! {
            incoming = socket.recv() => match incoming {
                None | Some(Ok(Message::Close(_))) => break,
                Some(Err(error)) => {
                    tracing::warn!(%error, "Graph update WebSocket receive failed");
                    break;
                }
                Some(Ok(Message::Ping(_) | Message::Pong(_))) => {}
                Some(Ok(_)) => {
                    tracing::warn!("Unexpected client payload on graph update WebSocket");
                    break;
                }
            },
            update = async {
                tokio::select! {
                    result = update_receiver.changed() => result,
                    _ = updates.tick() => Ok(()),
                }
            }, if watching => {
                if let Err(error) = update {
                    tracing::warn!(%error, "Graph update runtime watch channel closed");
                    break;
                }
                watching = if let Some(run_state) = &run_state {
                    match run_state.summary().await {
                        Ok(summary) => summary.complete_time.is_none()
                            && summary.cancelled_time.is_none()
                            && summary.error_time.is_none(),
                        Err(error) => {
                            tracing::error!(%error, "Failed to read graph update state");
                            if let Err(error) = socket.send(Message::Close(Some(CloseFrame {
                                code: 1011,
                                reason: "Failed to read workflow state".into(),
                            }))).await {
                                tracing::warn!(%error, "Graph update WebSocket close failed");
                            }
                            break;
                        }
                    }
                } else {
                    false
                };
                if let Err(error) = socket.send(Message::Text("refresh".into())).await {
                    tracing::warn!(%error, "Graph update WebSocket send failed");
                    break;
                }
            }
        }
    }
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
    let summaries = state.runtime_state.list_workflow_run_summaries().await?;

    let displays: Vec<WorkflowDisplay> = summaries
        .into_iter()
        .map(|s| {
            let (id_int, _) = s.run_id.as_u64_pair();
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
    if let Some(graph_data) = &state.display_graph {
        let mut graphs = HashMap::new();
        for loc_str in &query.locs {
            let location = parse_location(loc_str)?;
            let graph = super::nodes::build_graph_data_py_graph(graph_data, &location)?;
            graphs.insert(loc_str.clone(), graph);
        }
        return Ok(Json(GraphsResponse { graphs }));
    }

    // Once we get the actual workflow ID the logic needs to be reversed
    // Or the frontend needs to start submitting run_ids
    let run_state = state
        .runtime_state
        .load_workflow_run_state(run_id, 0)
        .await?;
    let workflow_id = run_state.workflow_id();

    tracing::info!("Listing nodes for {}", run_id);

    // TODO can we somehow avoid loading the entire graph (e.g. only the nested ones we need)
    let (_name, top_level_graph) = state.runtime_state.load_workflow(workflow_id).await?;

    let mut graphs = HashMap::new();
    for loc_str in &query.locs {
        let result = load_graph(&top_level_graph, loc_str).await?;
        let py_graph = match result {
            GraphLoadResult::Eval { graph, prefix } => {
                build_py_graph(&graph, run_state.as_ref(), &prefix, &state.asset_registry).await?
            }
            GraphLoadResult::LoopIterations {
                loop_node_location,
                subgraph,
            } => {
                build_loop_py_graph(
                    &loop_node_location,
                    &subgraph,
                    run_state.as_ref(),
                    &state.asset_registry,
                )
                .await?
            }
            GraphLoadResult::MapIterations {
                map_node_location,
                subgraph,
            } => {
                build_map_py_graph(
                    &map_node_location,
                    &subgraph,
                    run_state.as_ref(),
                    &state.asset_registry,
                )
                .await?
            }
        };
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
    if let Some(graph_data) = &state.display_graph {
        let location = parse_location(&location_str)?;
        return Ok(Json(super::nodes::graph_data_outputs(
            graph_data, &location,
        )?));
    }

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
    if let Some(graph_data) = &state.display_graph {
        let location = parse_location(&node_location_str)?;
        let outputs = super::nodes::graph_data_outputs(graph_data, &location)?;
        return Ok(Json(
            outputs
                .get(&port_name)
                .cloned()
                .unwrap_or(serde_json::Value::Null),
        ));
    }

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
    let (_name, top_level_graph) = state.runtime_state.load_workflow(workflow_id).await?;

    let loc = parse_location(&node_location_str)?;
    let node_location = loc.to_string();
    let parent_location = loc.parent();
    let result = load_graph(&top_level_graph, &parent_location.to_string()).await?;
    let py_graph = match result {
        GraphLoadResult::Eval { graph, prefix } => {
            build_py_graph(&graph, run_state.as_ref(), &prefix, &state.asset_registry).await?
        }
        GraphLoadResult::LoopIterations {
            loop_node_location,
            subgraph,
        } => {
            build_loop_py_graph(
                &loop_node_location,
                &subgraph,
                run_state.as_ref(),
                &state.asset_registry,
            )
            .await?
        }
        GraphLoadResult::MapIterations {
            map_node_location,
            subgraph,
        } => {
            build_map_py_graph(
                &map_node_location,
                &subgraph,
                run_state.as_ref(),
                &state.asset_registry,
            )
            .await?
        }
    };

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

/// Get the error logs for a specific node in a workflow run, returning the error detail as a string.
///
/// # Errors
///
/// Returns an internal server error if the workflow run state cannot be loaded or if the error logs cannot be loaded.
#[utoipa::path(
    get,
    path = "/workflows/{workflow_id}/nodes/{node_location_str}/errors",
    params(
        ("workflow_id" = Uuid, Path, description = "Run ID"),
        ("node_location_str" = String, Path, description = "Location string"),
    ),
    responses(
        (status = OK, description = "Error detail for the node"),
        (status = 500, description = "Error loading error detail"),
    )
)]
pub async fn get_node_errors(
    State(state): State<AppState>,
    Path((run_id, location_str)): Path<(Uuid, String)>,
) -> HandlerResult<Response> {
    let run_state = state
        .runtime_state
        .load_workflow_run_state(run_id, 0)
        .await?;

    let loc = parse_location(&location_str)?;
    let node_state = run_state.read(&loc).await?;
    Ok(node_state
        .error_detail
        .unwrap_or_else(|| "No logs available".to_string())
        .into_response())
}

/// Restart a task node and return the locations invalidated by the restart.
///
/// # Errors
///
/// Returns an internal server error if the workflow run or node cannot be restarted.
#[utoipa::path(
    post,
    path = "/workflows/{workflow_id}/nodes/{node_location_str}/restart",
    params(
        ("workflow_id" = Uuid, Path, description = "Run ID"),
        ("node_location_str" = String, Path, description = "Location string"),
    ),
    responses(
        (status = OK, body = Vec<String>),
        (status = 500, description = "Error restarting node"),
    )
)]
pub async fn restart_node(
    State(state): State<AppState>,
    Path((run_id, location_str)): Path<(Uuid, String)>,
) -> HandlerResult<Json<Vec<String>>> {
    if state.display_graph.is_some() {
        return Ok(Json(Vec::new()));
    }

    let location = parse_location(&location_str)?;
    let (_, invalidated) = state
        .runtime_state
        .restart_task(&state.asset_registry, run_id, 0, vec![location])
        .await?;

    Ok(Json(
        invalidated
            .into_iter()
            .map(|location| location.to_string())
            .collect(),
    ))
}

#[allow(unused)]
/// Get the logs for a specific node in a workflow run, returning the log detail as a string.
///
/// # Errors
///
/// Returns an internal server error if the workflow run state cannot be loaded or if the logs cannot be loaded.
#[utoipa::path(
    get,
    path = "/workflows/{workflow_id}/nodes/{node_location_str}/logs",
    params(
        ("workflow_id" = Uuid, Path, description = "Run ID"),
        ("node_location_str" = String, Path, description = "Location string"),
    ),
    responses(
        (status = OK, description = "Log detail for the node"),
        (status = 500, description = "Error loading log detail"),
    )
)]
pub async fn get_node_logs(
    State(state): State<AppState>,
    Path((run_id, location_str)): Path<(Uuid, String)>,
) -> HandlerResult<Response> {
    Ok("Not implemented".to_string().into_response())
}

#[allow(unused)]
/// Get the logs for a specific workflow run, returning the log detail as a string.
///
/// # Errors
///
/// Returns an internal server error if the workflow run state cannot be loaded or if the logs cannot be loaded.
#[utoipa::path(
    get,
    path = "/workflows/{workflow_id}/logs",
    params(
        ("workflow_id" = Uuid, Path, description = "Run ID"),
    ),
    responses(
        (status = OK, description = "Log detail for the workflow"),
        (status = 500, description = "Error loading log detail"),
    )
)]
pub async fn get_workflow_logs(
    State(state): State<AppState>,
    Path(run_id): Path<Uuid>,
) -> HandlerResult<Response> {
    Ok("Not implemented".to_string().into_response())
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{Router, routing::get};
    use axum_test::{TestServer, TestWebSocket};
    use rstest::rstest;
    use tokio::time::timeout;

    use crate::{
        event::{NodeEvent, NodeStatus, WorkflowRunEvent},
        graph::WorkflowGraph,
        state::{InMemoryRuntimeState, RuntimeState, SqliteRuntimeState},
    };

    async fn node_update_socket(
        runtime_state: Arc<dyn RuntimeState>,
        run_id: Uuid,
    ) -> (TestServer, TestWebSocket) {
        let state = AppState {
            update_receiver: runtime_state.listen(),
            runtime_state,
            asset_registry: Arc::new(HashMap::new()),
            display_graph: None,
        };
        let app = Router::new()
            .route(
                "/workflows/{workflow_id}/nodes/{node_location_str}",
                get(subscribe_node_updates),
            )
            .with_state(state);
        let server = TestServer::builder().http_transport().build(app);
        let socket = server
            .get_websocket(&format!("/workflows/{run_id}/nodes/-"))
            .await
            .into_websocket()
            .await;
        (server, socket)
    }

    #[rstest]
    #[case::in_memory(false)]
    #[case::sqlite(true)]
    #[tokio::test(flavor = "multi_thread")]
    async fn runtime_events_refresh_before_next_poll(#[case] sqlite: bool) -> miette::Result<()> {
        let runtime_state: Arc<dyn RuntimeState> = if sqlite {
            Arc::new(SqliteRuntimeState::try_new_in_memory().await?)
        } else {
            Arc::new(InMemoryRuntimeState::new())
        };
        let workflow_id = runtime_state
            .save_workflow(None, WorkflowGraph::new(std::iter::empty::<String>()))
            .await?;
        let run_state = runtime_state
            .new_workflow_run_state(workflow_id, HashMap::new())
            .await?;
        let (_server, mut socket) = node_update_socket(runtime_state, run_state.run_id()).await;
        timeout(
            Duration::from_secs(2),
            socket.assert_receive_text("refresh"),
        )
        .await
        .expect("Initial graph refresh was not sent");

        for event in [
            WorkflowRunEvent::NodeEvents(vec![NodeEvent {
                loc: Location::root(),
                name: None,
                status: NodeStatus::Running { state_update: None },
            }]),
            WorkflowRunEvent::Completed {},
        ] {
            run_state.write(event).await?;
            timeout(
                Duration::from_millis(500),
                socket.assert_receive_text("refresh"),
            )
            .await
            .expect("Runtime event did not trigger a refresh before the next poll");
        }

        assert!(
            timeout(Duration::from_millis(1100), socket.receive_message())
                .await
                .is_err(),
            "Completed workflows should stop sending refresh notifications"
        );
        socket.close().await;
        Ok(())
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn separate_sqlite_writer_refreshes_via_polling() -> miette::Result<()> {
        let directory = tempfile::tempdir().unwrap();
        let database_url = directory.path().join("state.sqlite");
        let database_url = database_url.to_str().unwrap();
        let writer = SqliteRuntimeState::try_new_with_url(database_url).await?;
        let workflow_id = writer
            .save_workflow(None, WorkflowGraph::new(std::iter::empty::<String>()))
            .await?;
        let run_state = writer
            .new_workflow_run_state(workflow_id, HashMap::new())
            .await?;
        let reader = Arc::new(SqliteRuntimeState::try_new_with_url(database_url).await?);
        let mut updates = reader.listen();
        updates.mark_unchanged();
        let (_server, mut socket) = node_update_socket(reader, run_state.run_id()).await;
        timeout(
            Duration::from_secs(2),
            socket.assert_receive_text("refresh"),
        )
        .await
        .expect("Initial graph refresh was not sent");

        run_state.write(WorkflowRunEvent::Completed {}).await?;
        assert!(!updates.has_changed().unwrap());
        timeout(
            Duration::from_secs(2),
            socket.assert_receive_text("refresh"),
        )
        .await
        .expect("External SQLite write was not detected by polling");
        assert!(
            timeout(Duration::from_millis(1100), socket.receive_message())
                .await
                .is_err(),
            "Externally completed workflows should stop sending refresh notifications"
        );
        socket.close().await;
        Ok(())
    }
}
