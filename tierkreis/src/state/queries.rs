/*!
This module defines the queries for reading the workflow state from the `SQlite` database.
*/
use std::collections::HashMap;
use std::hash::BuildHasher;

use bitvec::vec::BitVec;
use chrono::{DateTime, NaiveDateTime, Utc};
use diesel::sql_types::{Binary, Bool, Integer, Nullable, Text, Timestamp};
use diesel::sqlite::Sqlite;
use diesel::upsert::excluded;
use diesel::{
    BelongingToDsl, ExpressionMethods, NullableExpressionMethods, OptionalExtension, QueryDsl,
    SelectableHelper, define_sql_function,
};
use diesel_async::scoped_futures::ScopedFutureExt;
use diesel_async::{AsyncConnection, RunQueryDsl};
use miette::{IntoDiagnostic, WrapErr, miette};

use crate::asset_storage::AssetSpec;
use crate::location::Location;
use crate::state::interface::ExecutorDebugInformation;
use crate::state::models::{
    NewNodeOutput, NewWorkflow, NewWorkflowRun, NewWorkflowRunInput, NodeOutput, NodeState, UpsertNodeState, Workflow, WorkflowRun, WorkflowRunAttempt, WorkflowRunInput,
};

fn utc_timestamp(ts: NaiveDateTime) -> DateTime<Utc> {
    DateTime::<Utc>::from_naive_utc_and_offset(ts, Utc)
}

/// Read a workflow graph from the workflows table.
///
/// # Errors
///
/// Returns an error when the insert fails.
pub async fn read_workflow(
    conn: &mut impl AsyncConnection<Backend = Sqlite>,
    workflow_id: uuid::Uuid,
) -> miette::Result<Workflow> {
    use crate::state::schema::workflows::dsl as wf;

    let workflow = wf::workflows
        .find(workflow_id.to_string())
        .select(Workflow::as_select())
        .get_result(conn)
        .await
        .into_diagnostic()
        .wrap_err_with(|| miette!("Failed to select workflow with id: {}", workflow_id))?;

    Ok(workflow)
}

/// Insert a workflow graph to the workflows table.
///
/// # Errors
///
/// Returns an error when the insert fails.
pub async fn insert_workflow(
    conn: &mut impl AsyncConnection<Backend = Sqlite>,
    workflow: &NewWorkflow<'_>,
) -> diesel::result::QueryResult<()> {
    use crate::state::schema::workflows::dsl as wf;

    diesel::insert_into(wf::workflows)
        .values(workflow)
        .on_conflict(wf::id)
        .do_nothing()
        .execute(conn)
        .await?;

    Ok(())
}

/// Load the run-attempt state for a workflow run if it exists.
///
/// # Errors
///
/// If the run does not exists.
/// Returns an error when the connection pool cannot be accessed.
pub async fn read_workflow_run(
    conn: &mut impl AsyncConnection<Backend = Sqlite>,
    run_id: uuid::Uuid,
    attempt: u32,
) -> miette::Result<(WorkflowRun, WorkflowRunAttempt)> {
    use crate::state::schema::workflow_run_attempts::dsl as wra;
    use crate::state::schema::workflow_runs::dsl as wr;

    let attempt_i32 = i32::try_from(attempt)
        .into_diagnostic()
        .wrap_err_with(|| miette!("Attempt value {attempt} does not fit into i32"))?;

    wr::workflow_runs
        .filter(wr::id.eq(run_id.to_string()))
        .inner_join(wra::workflow_run_attempts)
        .filter(wra::attempt.eq(attempt_i32))
        .select((WorkflowRun::as_select(), WorkflowRunAttempt::as_select()))
        .first::<(WorkflowRun, WorkflowRunAttempt)>(conn)
        .await
        .into_diagnostic()
        .wrap_err_with(|| miette!("Failed to query workflow run"))
}

/// Insert a workflow run row.
///
/// # Errors
///
/// Returns an error when the insert fails.
pub async fn insert_workflow_run(
    conn: &mut impl AsyncConnection<Backend = Sqlite>,
    run: &NewWorkflowRun<'_>,
) -> diesel::result::QueryResult<WorkflowRun> {
    use crate::state::schema::workflow_run_attempts::dsl as wra;
    use crate::state::schema::workflow_runs::dsl as wr;

    let workflow_run = diesel::insert_into(wr::workflow_runs)
        .values(run)
        .on_conflict_do_nothing()
        .returning(WorkflowRun::as_returning())
        .get_result(conn)
        .await?;

    diesel::insert_into(wra::workflow_run_attempts)
        .values(wra::workflow_run_id.eq(&workflow_run.id))
        .execute(conn)
        .await?;

    Ok(workflow_run)
}

define_sql_function!(
    /// `COALESCE` for datetime data.
    #[sql_name = "coalesce"]
    fn coalesce_datetime(x: Nullable<Timestamp>, y: Nullable<Timestamp>) -> Nullable<Timestamp>;
);

define_sql_function!(
    /// `COALESCE` for boolean data.
    #[sql_name = "coalesce"]
    fn coalesce_bool(x: Nullable<Bool>, y: Nullable<Bool>) -> Nullable<Bool>;
);

define_sql_function!(
    /// `COALESCE` for integer data.
    #[sql_name = "coalesce"]
    fn coalesce_int(x: Nullable<Integer>, y: Nullable<Integer>) -> Nullable<Integer>;
);

define_sql_function!(
    /// `COALESCE` for textual data.
    #[sql_name = "coalesce"]
    fn coalesce_text(x: Nullable<Text>, y: Nullable<Text>) -> Nullable<Text>;
);

define_sql_function!(
    /// `COALESCE` for binary BLOB data.
    #[sql_name = "coalesce"]
    fn coalesce_blob(x: Nullable<Binary>, y: Nullable<Binary>) -> Nullable<Binary>;
);

define_sql_function!(
    /// `COALESCE` for binary BLOB data.
    fn changes() -> Integer;
);

define_sql_function!(
    /// `MAX` for integer data.
    #[sql_name = "max"]
    fn max_int(x: Nullable<Integer>, y: Nullable<Integer>) -> Nullable<Integer>;
);

/// Ensure a node-state row exists for the given run/location.
///
/// # Errors
///
/// Returns an error when the connection pool cannot be accessed or the insert
/// operation fails.
pub async fn update_node_state(
    conn: &mut impl AsyncConnection<Backend = Sqlite>,
    run_id: &str,
    attempt: i32,
    mut node_updates: Vec<UpsertNodeState>,
    node_outputs: Vec<(Location, NewNodeOutput<'_>)>,
) -> miette::Result<bool> {
    use crate::state::schema::node_states::dsl as ns;

    conn.transaction(|conn| {
        async move {
            merge_map_complete_fields(conn, run_id, attempt, &mut node_updates).await?;

            // TODO: We shouldn't need to loop if batch inserts for sqlite and diesel_async are patched.
            let mut rows_affected = 0;
            for node_update in node_updates {
                rows_affected += diesel::insert_into(ns::node_states)
                    .values(node_update)
                    .on_conflict((ns::run_id, ns::attempt, ns::node_location))
                    .do_update()
                    .set((
                        ns::scheduled_time.eq(coalesce_datetime(
                            ns::scheduled_time,
                            excluded(ns::scheduled_time),
                        )),
                        ns::queued_time.eq(coalesce_datetime(
                            ns::queued_time,
                            excluded(ns::queued_time),
                        )),
                        ns::running_time.eq(coalesce_datetime(
                            ns::running_time,
                            excluded(ns::running_time),
                        )),
                        ns::complete_time.eq(coalesce_datetime(
                            ns::complete_time,
                            excluded(ns::complete_time),
                        )),
                        ns::cancelled_time.eq(coalesce_datetime(
                            ns::cancelled_time,
                            excluded(ns::cancelled_time),
                        )),
                        ns::error_time
                            .eq(coalesce_datetime(ns::error_time, excluded(ns::error_time))),
                        ns::cond.eq(coalesce_bool(ns::cond, excluded(ns::cond))),
                        // Ordering intentionally reversed as we always want the excluded loop_index
                        // if it is not NULL.
                        ns::loop_index.eq(coalesce_int(excluded(ns::loop_index), ns::loop_index)),
                        ns::map_size.eq(coalesce_int(ns::map_size, excluded(ns::map_size))),
                        // Ordering intentionally reversed as we always want the excluded map_completed
                        // if it is not NULL.
                        ns::map_completed.eq(coalesce_blob(
                            excluded(ns::map_completed),
                            ns::map_completed,
                        )),
                        ns::error.eq(coalesce_text(ns::error, excluded(ns::error))),
                        ns::error_detail
                            .eq(coalesce_text(ns::error_detail, excluded(ns::error_detail))),
                    ))
                    .execute(conn)
                    .await?;
            }

            insert_outputs(conn, run_id, attempt, node_outputs).await?;

            Ok::<_, diesel::result::Error>(rows_affected > 0)
        }
        .scope_boxed()
    })
    .await
    .into_diagnostic()
}

async fn merge_map_complete_fields(
    conn: &mut impl AsyncConnection<Backend = Sqlite>,
    run_id: &str,
    attempt: i32,
    node_updates: &mut [UpsertNodeState],
) -> Result<(), diesel::result::Error> {
    use crate::state::schema::node_states::dsl as ns;

    let map_complete_locations: Vec<_> = node_updates
        .iter()
        .filter(|node_update| node_update.map_completed.is_some())
        .map(|node_update| &node_update.node_location)
        .collect();

    if map_complete_locations.is_empty() {
        return Ok(());
    }

    let existing_map_locations: HashMap<Location, Vec<u8>> = ns::node_states
        .select((ns::node_location, ns::map_completed.assume_not_null()))
        .filter(ns::run_id.eq(run_id))
        .filter(ns::attempt.eq(attempt))
        .filter(ns::map_completed.is_not_null())
        .filter(ns::node_location.eq_any(map_complete_locations))
        .get_results(conn)
        .await?
        .into_iter()
        .collect();

    for (node_location, node_update) in node_updates.iter_mut().filter_map(|node_update| {
        node_update
            .map_completed
            .as_mut()
            .map(|map_completed| (&node_update.node_location, map_completed))
    }) {
        if let Some(existing) = existing_map_locations.get(node_location) {
            node_update
                .iter_mut()
                .zip(existing)
                .for_each(|(x, y)| *x |= y);
        }
    }

    Ok(())
}

/// Read the persisted node state for a workflow run at a given location.
///
/// # Errors
///
/// Returns an error when the connection pool cannot be accessed or the node state
/// lookup fails.
pub async fn read_node_state(
    conn: &mut impl AsyncConnection<Backend = Sqlite>,
    run_id: uuid::Uuid,
    attempt: u32,
    loc: &Location,
) -> miette::Result<crate::state::interface::NodeState> {
    use crate::state::schema::node_states::dsl as ns;

    let attempt_i32 = i32::try_from(attempt)
        .into_diagnostic()
        .wrap_err_with(|| miette!("Attempt value {attempt} does not fit into i32"))?;
    let db_node = ns::node_states
        .filter(ns::run_id.eq(run_id.to_string()))
        .filter(ns::attempt.eq(attempt_i32))
        .filter(ns::node_location.eq(loc.clone()))
        .first::<NodeState>(conn)
        .await
        .optional()
        .into_diagnostic()
        .wrap_err_with(|| {
            miette!(
                "Failed to query node state for run {run_id} attempt {attempt} location {loc:?}"
            )
        })?;

    if let Some(db_node) = db_node {
        let loop_index = db_node
            .loop_index
            .map(|idx| {
                u32::try_from(idx)
                    .into_diagnostic()
                    .wrap_err_with(||miette!(
                        "Stored loop index {idx} is invalid for run {run_id} attempt {attempt} location {:?}",
                        loc
                    ))
            })
            .transpose()?;
        let map_completed = db_node
            .map_completed
            .as_deref()
            .map(|x| {
                let mut bits = BitVec::from_slice(x);
                bits.truncate(
                    db_node
                        .map_size
                        .ok_or_else(|| miette!("Could not get map size from node state"))?
                        .try_into()
                        .into_diagnostic()?,
                );
                Ok::<_, miette::Report>(bits)
            })
            .transpose()?;

        let outputs = read_outputs(conn, &db_node).await?;

        Ok(crate::state::interface::NodeState {
            scheduled_time: db_node.scheduled_time.map(utc_timestamp),
            queued_time: db_node.queued_time.map(utc_timestamp),
            running_time: db_node.running_time.map(utc_timestamp),
            complete_time: db_node.complete_time.map(utc_timestamp),
            cancelled_time: db_node.cancelled_time.map(utc_timestamp),
            error_time: db_node.error_time.map(utc_timestamp),
            cond: db_node.cond,
            loop_index,
            map_completed,
            error: db_node.error.clone(),
            error_detail: db_node.error_detail.clone(),
            outputs,
        })
    } else {
        Ok(crate::state::interface::NodeState::default())
    }
}

/// Read the persisted node state for a workflow run at multiple locations.
///
/// # Errors
///
/// Returns an error when the connection pool cannot be accessed or the node state
/// lookup fails.
pub async fn read_node_states(
    conn: &mut impl AsyncConnection<Backend = Sqlite>,
    run_id: uuid::Uuid,
    attempt: u32,
    locations: &mut (dyn Iterator<Item = Location> + Send),
) -> miette::Result<HashMap<Location, crate::state::interface::NodeState>> {
    use crate::state::schema::node_states::dsl as ns;

    let mut states = HashMap::new();
    let attempt_i32 = i32::try_from(attempt)
        .into_diagnostic()
        .wrap_err_with(|| miette!("Attempt value {attempt} does not fit into i32"))?;
    let db_nodes = ns::node_states
        .filter(ns::run_id.eq(run_id.to_string()))
        .filter(ns::attempt.eq(attempt_i32))
        .filter(ns::node_location.eq_any(locations))
        .get_results::<NodeState>(conn)
        .await
        .into_diagnostic()
        .wrap_err_with(|| {
            miette!("Failed to query node state for run {run_id} attempt {attempt}")
        })?;

    for db_node in db_nodes {
        let loop_index = db_node
            .loop_index
            .map(|idx| {
                u32::try_from(idx)
                    .into_diagnostic()
                    .wrap_err_with(||miette!(
                        "Stored loop index {idx} is invalid for run {run_id} attempt {attempt}",
                    ))
            })
            .transpose()?;
        let map_completed = db_node
            .map_completed
            .as_deref()
            .map(|x| {
                let mut bits = BitVec::from_slice(x);
                bits.truncate(
                    db_node
                        .map_size
                        .ok_or_else(|| miette!("Could not get map size from node state"))?
                        .try_into()
                        .into_diagnostic()?,
                );
                Ok::<_, miette::Report>(bits)
            })
            .transpose()?;

        let outputs = read_outputs(conn, &db_node).await?;

        states.insert(
            db_node.node_location,
            crate::state::interface::NodeState {
                scheduled_time: db_node.scheduled_time.map(utc_timestamp),
                queued_time: db_node.queued_time.map(utc_timestamp),
                running_time: db_node.running_time.map(utc_timestamp),
                complete_time: db_node.complete_time.map(utc_timestamp),
                cancelled_time: db_node.cancelled_time.map(utc_timestamp),
                error_time: db_node.error_time.map(utc_timestamp),
                cond: db_node.cond,
                loop_index,
                map_completed,
                error: db_node.error.clone(),
                error_detail: db_node.error_detail.clone(),
                outputs,
            },
        );
    }

    Ok(states)
}

/// Insert persisted workflow inputs for a workflow run.
///
/// # Errors
///
/// Returns an error when the connection pool cannot be accessed or the workflow run
/// input inserts fail.
pub async fn insert_workflow_run_inputs(
    conn: &mut impl AsyncConnection<Backend = Sqlite>,
    workflow_inputs: impl Iterator<Item = NewWorkflowRunInput<'_>>,
) -> diesel::result::QueryResult<()> {
    use crate::state::schema::workflow_run_inputs::dsl as wri;

    for workflow_input in workflow_inputs {
        diesel::insert_into(wri::workflow_run_inputs)
            .values(workflow_input)
            .execute(conn)
            .await?;
    }

    Ok(())
}

/// Read the persisted workflow inputs for a workflow run.
///
/// # Errors
///
/// Returns an error when the connection pool cannot be accessed or the workflow run
/// inputs lookup fails.
pub async fn read_workflow_run_inputs(
    conn: &mut impl AsyncConnection<Backend = Sqlite>,
    workflow_run_id: &str,
) -> miette::Result<HashMap<String, AssetSpec>> {
    use crate::state::schema::workflow_run_inputs::dsl as wri;

    let inputs = wri::workflow_run_inputs
        .select(WorkflowRunInput::as_select())
        .filter(wri::workflow_run_id.eq(workflow_run_id))
        .get_results(conn)
        .await
        .into_diagnostic()?;

    let inputs = inputs
        .into_iter()
        .map(|input| {
            let asset = AssetSpec {
                asset_key: input.asset_key.parse().into_diagnostic()?,
                kind: input.asset_kind.parse()?,
                storage_name: input.storage_name,
            };

            Ok((input.name, asset))
        })
        .collect::<miette::Result<HashMap<_, _>>>()?;

    Ok(inputs)
}

async fn insert_outputs(
    conn: &mut impl AsyncConnection<Backend = Sqlite>,
    run_id: &str,
    attempt: i32,
    node_outputs: Vec<(Location, NewNodeOutput<'_>)>,
) -> Result<(), diesel::result::Error> {
    use crate::state::schema::node_outputs::dsl as no;
    use crate::state::schema::node_states::dsl as ns;

    if !node_outputs.is_empty() {
        let locations = node_outputs.iter().map(|(x, _)| x);
        let node_state_ids: HashMap<Location, i32> = ns::node_states
            .select((ns::node_location, ns::id))
            .filter(ns::run_id.eq(run_id))
            .filter(ns::attempt.eq(attempt))
            .filter(ns::node_location.eq_any(locations))
            .get_results(conn)
            .await?
            .into_iter()
            .collect();

        let db_outputs: Vec<_> = node_outputs
            .into_iter()
            .map(|(location, node_output)| {
                (
                    no::node_state_id.eq(node_state_ids.get(&location).unwrap()),
                    node_output,
                )
            })
            .collect();

        // TODO: We shouldn't need to loop if batch inserts for sqlite and diesel_async are patched.
        for db_output in db_outputs {
            diesel::insert_into(no::node_outputs)
                .values(db_output)
                .on_conflict((no::node_state_id, no::name)) // TODO: Might need a unique index?
                .do_nothing()
                .execute(conn)
                .await?;
        }
    }
    Ok(())
}

async fn read_outputs(
    conn: &mut impl AsyncConnection<Backend = Sqlite>,
    db_node: &NodeState,
) -> Result<Option<HashMap<String, AssetSpec>>, miette::Error> {
    let outputs = if db_node.complete_time.is_some() {
        let db_outputs: Vec<NodeOutput> = NodeOutput::belonging_to(db_node)
            .get_results(conn)
            .await
            .into_diagnostic()?;
        let outputs: HashMap<String, AssetSpec> = db_outputs
            .into_iter()
            .map(|db_output| {
                Ok::<_, miette::Report>((
                    db_output.name,
                    AssetSpec {
                        kind: db_output.asset_kind.parse()?,
                        storage_name: db_output.storage_name,
                        asset_key: db_output.asset_key.parse().into_diagnostic()?,
                    },
                ))
            })
            .collect::<Result<_, _>>()?;
        Some(outputs)
    } else {
        None
    };

    Ok(outputs)
}

define_sql_function!(
    /// Patch a jsonb BLOB with a jsonb format patch, returning the patched copy.
    fn jsonb_patch(t: Binary, p: Binary) -> Binary;
);

/// Merge additional metadata into the persisted run metadata for a workflow run.
///
/// # Errors
///
/// Returns an error when the connection pool cannot be accessed, the run lookup
/// fails, metadata JSON cannot be parsed or serialized, or the update fails.
pub async fn add_run_attempt_metadata<S: BuildHasher>(
    conn: &mut impl AsyncConnection<Backend = Sqlite>,
    run_id: uuid::Uuid,
    attempt: u32,
    new_metadata: HashMap<String, String, S>,
) -> miette::Result<()> {
    use crate::state::schema::workflow_run_attempts::dsl as wra;

    let attempt_i32 = i32::try_from(attempt)
        .into_diagnostic()
        .wrap_err_with(|| miette!("Attempt value {attempt} does not fit into i32"))?;

    let run_id_str = run_id.to_string();

    let new_metadata = serde_json::to_vec(&new_metadata).into_diagnostic()?;

    diesel::update(
        wra::workflow_run_attempts
            .filter(wra::workflow_run_id.eq(run_id_str))
            .filter(wra::attempt.eq(attempt_i32)),
    )
    .set(wra::run_metadata.eq(jsonb_patch(wra::run_metadata, new_metadata)))
    .execute(conn)
    .await
    .into_diagnostic()
    .wrap_err_with(|| {
        miette!(
            "Failed to update workflow run metadata for run {} attempt {}",
            run_id,
            attempt
        )
    })?;

    Ok(())
}

define_sql_function!(
    /// Validate that x is valid JSON and return the string representation.
    fn json(x: Binary) -> Text;
);

define_sql_function!(
    /// Convert JSON text bytes to SQLite JSONB.
    fn jsonb(x: Binary) -> Binary;
);

/// Read the persisted metadata for a workflow run.
///
/// # Errors
///
/// Returns an error when the connection pool cannot be accessed, the run lookup
/// fails, or the metadata JSON cannot be parsed.
pub async fn read_run_attempt_metadata(
    conn: &mut impl AsyncConnection<Backend = Sqlite>,
    run_id: uuid::Uuid,
    attempt: u32,
) -> miette::Result<HashMap<String, String>> {
    use crate::state::schema::workflow_run_attempts::dsl as wra;

    let attempt_i32 = i32::try_from(attempt)
        .into_diagnostic()
        .wrap_err_with(|| miette!("Attempt value {attempt} does not fit into i32"))?;

    let run_id_str = run_id.to_string();

    let metadata = wra::workflow_run_attempts
        .filter(wra::workflow_run_id.eq(&run_id_str))
        .filter(wra::attempt.eq(attempt_i32))
        .select(json(wra::run_metadata))
        .first::<String>(conn)
        .await
        .into_diagnostic()
        .wrap_err_with(|| miette!("Failed to query workflow run for metadata update"))?;

    let metadata = serde_json::from_str::<HashMap<String, String>>(&metadata)
        .into_diagnostic()
        .wrap_err_with(|| {
            miette!(
                "Failed to parse existing run metadata JSON for run {}",
                run_id_str
            )
        })?;

    Ok(metadata)
}

/// A summarized view of a workflow run for display in the visualizer.
#[derive(Debug, Clone)]
pub struct WorkflowRunSummary {
    /// The run identifier.
    pub run_id: uuid::Uuid,
    /// The attempt number.
    pub attempt: u32,
    /// The workflow graph identifier.
    pub workflow_id: uuid::Uuid,
    /// An optional human-readable name taken from the workflow definition.
    pub name: Option<String>,
    /// The time the run was started, if available.
    pub started_time: Option<chrono::DateTime<chrono::Utc>>,
    /// The terminal status string stored in the run row.
    pub status: Option<String>,
    /// Locations of nodes that have errored in this run.
    pub errored_locations: Vec<Location>,
}

/// List all workflow runs with summary information for display in the visualizer.
///
/// # Errors
///
/// Returns an error when the connection pool cannot be accessed or the queries fail.
pub async fn list_workflow_run_summaries(
    conn: &mut impl AsyncConnection<Backend = Sqlite>,
) -> miette::Result<Vec<WorkflowRunSummary>> {
    use crate::state::schema::node_states::dsl as ns;
    use crate::state::schema::workflow_run_attempts::dsl as wra;
    use crate::state::schema::workflow_runs::dsl as wr;
    use crate::state::schema::workflows::dsl as wf;

    let runs: Vec<(WorkflowRun, WorkflowRunAttempt, Option<String>)> = wr::workflow_runs
        .inner_join(wra::workflow_run_attempts)
        .left_join(wf::workflows)
        .select((
            WorkflowRun::as_select(),
            WorkflowRunAttempt::as_select(),
            wf::name.nullable(),
        ))
        .order(wra::started_time.asc())
        .get_results(conn)
        .await
        .into_diagnostic()
        .wrap_err("Failed to list workflow runs")?;

    let mut summaries = Vec::with_capacity(runs.len());
    for (run, run_attempt, workflow_name) in runs {
        let run_id: uuid::Uuid = run
            .id
            .parse()
            .into_diagnostic()
            .wrap_err_with(|| miette!("Invalid run UUID: {}", run.id))?;
        let workflow_id: uuid::Uuid = run
            .workflow_id
            .parse()
            .into_diagnostic()
            .wrap_err_with(|| miette!("Invalid workflow UUID: {}", run.workflow_id))?;
        let attempt = u32::try_from(run_attempt.attempt)
            .into_diagnostic()
            .wrap_err_with(|| miette!("Invalid attempt value: {}", run_attempt.attempt))?;

        let errored_locations: Vec<Location> = ns::node_states
            .select(ns::node_location)
            .filter(ns::run_id.eq(&run.id))
            .filter(ns::attempt.eq(run_attempt.attempt))
            .filter(ns::error_time.is_not_null())
            .get_results(conn)
            .await
            .into_diagnostic()
            .wrap_err_with(|| miette!("Failed to list errored nodes for run {}", run.id))?;

        summaries.push(WorkflowRunSummary {
            run_id,
            attempt,
            workflow_id,
            name: workflow_name,
            started_time: run_attempt.started_time.map(utc_timestamp),
            status: run_attempt.status,
            errored_locations,
        });
    }

    Ok(summaries)
}

/// Set executor debug information for a node in a specific run attempt.
///
/// # Errors
///
/// Returns an error when node state cannot be found or the insert fails.
pub async fn add_executor_debug_information(
    conn: &mut impl AsyncConnection<Backend = Sqlite>,
    run_id: uuid::Uuid,
    attempt: u32,
    information: ExecutorDebugInformation,
) -> miette::Result<()> {
    use crate::state::schema::executor_debug::dsl as ed;

    let node_state_id =
        resolve_node_state_id(conn, run_id, attempt, &information.node_location).await?;
    let resources = serde_json::to_vec(&information.resources)
        .into_diagnostic()
        .wrap_err("Failed to serialize executor resources")?;
    let environment = serde_json::to_vec(&information.environment)
        .into_diagnostic()
        .wrap_err("Failed to serialize executor environment")?;

    diesel::insert_into(ed::executor_debug)
        .values((
            ed::node_state_id.eq(node_state_id),
            ed::executor_name.eq(information.executor_name),
            ed::worker_name.eq(information.worker_name),
            ed::task_name.eq(information.task_name),
            ed::resources.eq(jsonb(resources)),
            ed::environment.eq(jsonb(environment)),
            ed::internal_id.eq(information.internal_id),
        ))
        .on_conflict(ed::node_state_id)
        .do_nothing()
        .execute(conn)
        .await
        .into_diagnostic()
        .wrap_err_with(|| {
            miette!(
                "Failed to insert executor debug information for run {} attempt {} location {:?}",
                run_id,
                attempt,
                information.node_location
            )
        })?;

    Ok(())
}

/// Set executor internal identifier for a node in a specific run attempt.
///
/// # Errors
///
/// Returns an error when node state cannot be found or the update fails.
pub async fn set_executor_debug_internal_id(
    conn: &mut impl AsyncConnection<Backend = Sqlite>,
    run_id: uuid::Uuid,
    attempt: u32,
    node_location: &Location,
    internal_id: &str,
) -> miette::Result<()> {
    use crate::state::schema::executor_debug::dsl as ed;

    let node_state_id = resolve_node_state_id(conn, run_id, attempt, node_location).await?;
    diesel::update(ed::executor_debug.filter(ed::node_state_id.eq(node_state_id)))
        .set(ed::internal_id.eq(Some(internal_id.to_string())))
        .execute(conn)
        .await
        .into_diagnostic()
        .wrap_err_with(|| {
            miette!(
                "Failed to set executor internal id for run {} attempt {} location {:?}",
                run_id,
                attempt,
                node_location
            )
        })?;

    Ok(())
}

/// Read executor debug information for a node in a specific run attempt.
///
/// # Errors
///
/// Returns an error when node state cannot be found, stored JSON cannot be parsed,
/// or the query fails.
pub async fn read_executor_debug_information(
    conn: &mut impl AsyncConnection<Backend = Sqlite>,
    run_id: uuid::Uuid,
    attempt: u32,
    node_location: &Location,
) -> miette::Result<ExecutorDebugInformation> {
    use crate::state::schema::executor_debug::dsl as ed;

    let node_state_id = resolve_node_state_id(conn, run_id, attempt, node_location).await?;
    let (executor_name, worker_name, task_name, resources, environment, internal_id) = ed::executor_debug
        .filter(ed::node_state_id.eq(node_state_id))
        .select((
            ed::executor_name,
            ed::worker_name,
            ed::task_name,
            json(ed::resources),
            json(ed::environment),
            ed::internal_id,
        ))
        .first::<(String, String, String, String, String, Option<String>)>(conn)
        .await
        .into_diagnostic()
        .wrap_err_with(|| {
            miette!(
                "Failed to query executor debug information for run {} attempt {} location {:?}",
                run_id,
                attempt,
                node_location
            )
        })?;

    let resources = serde_json::from_str(&resources)
        .into_diagnostic()
        .wrap_err("Failed to parse executor debug resources JSON")?;
    let environment = serde_json::from_str(&environment)
        .into_diagnostic()
        .wrap_err("Failed to parse executor debug environment JSON")?;

    Ok(ExecutorDebugInformation {
        run_id,
        attempt,
        node_location: node_location.clone(),
        executor_name,
        worker_name,
        task_name,
        resources,
        environment,
        internal_id,
    })
}

async fn resolve_node_state_id(
    conn: &mut impl AsyncConnection<Backend = Sqlite>,
    run_id: uuid::Uuid,
    attempt: u32,
    node_location: &Location,
) -> miette::Result<i32> {
    use crate::state::schema::node_states::dsl as ns;

    let attempt_i32 = i32::try_from(attempt)
        .into_diagnostic()
        .wrap_err_with(|| miette!("Attempt value {attempt} does not fit into i32"))?;

    ns::node_states
        .select(ns::id)
        .filter(ns::run_id.eq(run_id.to_string()))
        .filter(ns::attempt.eq(attempt_i32))
        .filter(ns::node_location.eq(node_location.clone()))
        .first::<i32>(conn)
        .await
        .into_diagnostic()
        .wrap_err_with(|| {
            miette!(
                "Could not find node state for run {} attempt {} location {:?}",
                run_id,
                attempt,
                node_location
            )
        })

}
