use super::models::{
    AppState, AttemptSummary, ExecutorSummary, MonitoringSummary, NewRunRequest, NewRunResponse,
    RunSummary, RuntimeInfo, RuntimeMetadata, TraceSpan, WorkflowDisplay, WorkflowPorts,
    WorkflowSummary,
};

use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use axum_extra::extract::Query;
use miette::IntoDiagnostic;
use std::collections::HashMap;

use crate::{
    location::Location,
    server::{
        models::{GraphsQuery, GraphsResponse, HandlerResult},
        nodes::{
            GraphLoadResult, build_loop_py_graph, build_map_py_graph, build_py_graph, load_graph,
            try_load_output_value, try_load_outputs,
        },
    },
};
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

/// Compute aggregate runtime statistics for the monitoring dashboard.
///
/// # Errors
///
/// Returns an internal server error if the database query fails.
#[utoipa::path(
    get,
    path = "/monitor",
    responses((status = OK, body = MonitoringSummary))
)]
pub async fn get_monitoring_summary(
    State(state): State<AppState>,
) -> HandlerResult<Json<MonitoringSummary>> {
    let summaries = state.runtime_state.list_workflow_run_summaries().await?;
    let node_stats = state.runtime_state.node_stats().await?;

    let total_runs = summaries.len();
    let runs_with_errors = summaries
        .iter()
        .filter(|s| !s.errored_locations.is_empty())
        .count();
    let active_runs = summaries
        .into_iter()
        .filter(|s| {
            s.complete_time.is_none() && s.cancelled_time.is_none() && s.error_time.is_none()
        })
        .map(|s| super::models::ActiveRun {
            run_id: s.run_id,
            attempt: s.attempt,
            name: s.name,
            started_time: s.started_time.map_or_else(String::new, |t| t.to_rfc3339()),
        })
        .collect();

    Ok(Json(MonitoringSummary {
        active_runs,
        total_runs,
        runs_with_errors,
        tasks_running: node_stats.tasks_running,
        tasks_completed: node_stats.tasks_completed,
        tasks_errored: node_stats.tasks_errored,
        tasks_cancelled: node_stats.tasks_cancelled,
        avg_task_duration_seconds: node_stats.avg_duration_seconds,
    }))
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
    let run_state = state
        .runtime_state
        .load_workflow_run_state(run_id, 0)
        .await?;

    let loc = parse_location(&location_str)?;
    let node_state = run_state.read(&loc).await?;
    Ok(node_state
        .logs
        .unwrap_or_else(|| "No logs available".to_string())
        .into_response())
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

/// List every Workflow, grouped by workflow, then by run, then by attempt.
///
/// A Workflow is a graph structure; a run is a Workflow tied to a specific set
/// of inputs; an attempt is a single execution of a run (restarts of the same
/// run create additional attempts).
///
/// # Errors
///
/// Returns an internal server error if the database query fails.
///
/// # Panics
///
/// Never panics in practice: the `expect` calls only follow an unconditional push.
#[utoipa::path(
    get,
    path = "/monitor/workflows",
    responses((status = OK, body = Vec<WorkflowSummary>))
)]
pub async fn get_workflows_summary(
    State(state): State<AppState>,
) -> HandlerResult<Json<Vec<WorkflowSummary>>> {
    let all_workflows = state.runtime_state.list_workflows().await?;
    let summaries = state.runtime_state.list_workflow_run_summaries().await?;

    // Seed with every saved Workflow (including ones with no runs yet) so
    // they still show up in the monitor.
    let mut workflows: Vec<WorkflowSummary> = all_workflows
        .into_iter()
        .map(|(workflow_id, name)| WorkflowSummary {
            workflow_id,
            name,
            runs: Vec::new(),
        })
        .collect();

    for s in summaries {
        let workflow = if let Some(w) = workflows
            .iter_mut()
            .find(|w| w.workflow_id == s.workflow_id)
        {
            w
        } else {
            workflows.push(WorkflowSummary {
                workflow_id: s.workflow_id,
                name: s.name.clone(),
                runs: Vec::new(),
            });
            workflows.last_mut().expect("just pushed")
        };

        let run = if let Some(r) = workflow.runs.iter_mut().find(|r| r.run_id == s.run_id) {
            r
        } else {
            workflow.runs.push(RunSummary {
                run_id: s.run_id,
                attempts: Vec::new(),
            });
            workflow.runs.last_mut().expect("just pushed")
        };

        run.attempts.push(AttemptSummary {
            attempt: s.attempt,
            started_time: s.started_time.map_or_else(String::new, |t| t.to_rfc3339()),
            complete_time: s.complete_time.map(|t| t.to_rfc3339()),
            cancelled_time: s.cancelled_time.map(|t| t.to_rfc3339()),
            error_time: s.error_time.map(|t| t.to_rfc3339()),
            errored_locations: s
                .errored_locations
                .iter()
                .map(std::string::ToString::to_string)
                .collect(),
        });
    }

    Ok(Json(workflows))
}

/// Describe the runtime's version and the Executors available to it,
/// including their Workers and resources (e.g. HPC node counts).
///
/// # Errors
///
/// Returns an internal server error if any Executor cannot be queried.
#[utoipa::path(
    get,
    path = "/runtime",
    responses((status = OK, body = RuntimeInfo))
)]
pub async fn get_runtime_info(State(state): State<AppState>) -> HandlerResult<Json<RuntimeInfo>> {
    let executors = state
        .runtime
        .describe_executors()
        .await?
        .into_iter()
        .map(|info| ExecutorSummary {
            name: info.name,
            kind: info.kind,
            worker_count: info.worker_count,
            details: info.details,
        })
        .collect();

    Ok(Json(RuntimeInfo {
        version: env!("CARGO_PKG_VERSION").to_string(),
        executors,
    }))
}

/// Get the execution trace (every recorded Node state, in scheduling order)
/// for a specific Workflow run attempt.
///
/// # Errors
///
/// Returns an internal server error if the workflow run state cannot be loaded.
#[utoipa::path(
    get,
    path = "/runs/{run_id}/attempts/{attempt}/trace",
    params(
        ("run_id" = Uuid, Path, description = "Run ID"),
        ("attempt" = u32, Path, description = "Attempt number"),
    ),
    responses((status = OK, body = Vec<TraceSpan>))
)]
pub async fn get_run_trace(
    State(state): State<AppState>,
    Path((run_id, attempt)): Path<(Uuid, u32)>,
) -> HandlerResult<Json<Vec<TraceSpan>>> {
    let run_state = state
        .runtime_state
        .load_workflow_run_state(run_id, attempt)
        .await?;
    let states = run_state.read_all().await?;

    let spans = states
        .into_iter()
        .map(|(location, node_state)| TraceSpan {
            location: location.to_string(),
            status: super::models::node_status_from_state(&node_state),
            scheduled_time: node_state.scheduled_time.map(|t| t.to_rfc3339()),
            queued_time: node_state.queued_time.map(|t| t.to_rfc3339()),
            running_time: node_state.running_time.map(|t| t.to_rfc3339()),
            complete_time: node_state.complete_time.map(|t| t.to_rfc3339()),
            error_time: node_state.error_time.map(|t| t.to_rfc3339()),
            cancelled_time: node_state.cancelled_time.map(|t| t.to_rfc3339()),
            error: node_state.error,
        })
        .collect();

    Ok(Json(spans))
}

/// Start a new Workflow run with the given inputs.
///
/// # Errors
///
/// Returns an internal server error if an input cannot be serialized or the
/// runtime state cannot be written to.
#[utoipa::path(
    post,
    path = "/workflows/{workflow_id}/runs",
    params(("workflow_id" = Uuid, Path, description = "Workflow ID")),
    responses((status = OK, body = NewRunResponse))
)]
pub async fn start_new_run(
    State(state): State<AppState>,
    Path(workflow_id): Path<Uuid>,
    Json(request): Json<NewRunRequest>,
) -> HandlerResult<Json<NewRunResponse>> {
    let mut inputs = HashMap::new();
    for (name, value) in request.inputs {
        inputs.insert(name, serde_json::to_vec(&value).into_diagnostic()?);
    }

    let (run_id, attempt) = state.runtime.start_new_run(workflow_id, inputs).await?;
    Ok(Json(NewRunResponse { run_id, attempt }))
}

/// List the names of the top-level input ports of a Workflow, so that a
/// caller can be prompted for the right input names before starting a run.
///
/// # Errors
///
/// Returns an internal server error if the Workflow cannot be loaded.
#[utoipa::path(
    get,
    path = "/workflows/{workflow_id}/input_names",
    params(("workflow_id" = Uuid, Path, description = "Workflow ID")),
    responses((status = OK, body = Vec<String>))
)]
pub async fn get_workflow_input_names(
    State(state): State<AppState>,
    Path(workflow_id): Path<Uuid>,
) -> HandlerResult<Json<Vec<String>>> {
    let (_name, graph) = state.runtime_state.load_workflow(workflow_id).await?;

    let mut names: Vec<String> = graph
        .node_ids()
        .filter_map(|n| match graph.node_definition(n) {
            Some(crate::graph::NodeDefinition::Input { name }) => Some(name.clone()),
            _ => None,
        })
        .collect();
    names.sort();

    Ok(Json(names))
}

/// List the names of both the top-level input and output ports of a
/// Workflow, e.g. so the graph builder can show a subgraph reference node
/// with the right number of handles.
///
/// # Errors
///
/// Returns an internal server error if the Workflow cannot be loaded.
#[utoipa::path(
    get,
    path = "/workflows/{workflow_id}/ports",
    params(("workflow_id" = Uuid, Path, description = "Workflow ID")),
    responses((status = OK, body = WorkflowPorts))
)]
pub async fn get_workflow_ports(
    State(state): State<AppState>,
    Path(workflow_id): Path<Uuid>,
) -> HandlerResult<Json<WorkflowPorts>> {
    let (_name, graph) = state.runtime_state.load_workflow(workflow_id).await?;

    let mut inputs: Vec<String> = graph
        .node_ids()
        .filter_map(|n| match graph.node_definition(n) {
            Some(crate::graph::NodeDefinition::Input { name }) => Some(name.clone()),
            _ => None,
        })
        .collect();
    inputs.sort();

    let mut outputs: Vec<String> = graph
        .input_names(graph.output_idx())?
        .cloned()
        .collect();
    outputs.sort();

    Ok(Json(WorkflowPorts { inputs, outputs }))
}

/// Start a new attempt for an existing run, reusing its original inputs.
///
/// # Errors
///
/// Returns an internal server error if the run does not exist or the runtime
/// state cannot be written to.
#[utoipa::path(
    post,
    path = "/runs/{run_id}/attempts",
    params(("run_id" = Uuid, Path, description = "Run ID")),
    responses((status = OK, body = NewRunResponse))
)]
pub async fn start_new_attempt(
    State(state): State<AppState>,
    Path(run_id): Path<Uuid>,
) -> HandlerResult<Json<NewRunResponse>> {
    let attempt = state.runtime.create_next_attempt(run_id).await?;
    Ok(Json(NewRunResponse { run_id, attempt }))
}
