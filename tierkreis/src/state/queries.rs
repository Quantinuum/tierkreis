/*!
This module defines the queries for reading the workflow state from the `SQlite` database.
*/
use std::collections::HashMap;
use std::hash::BuildHasher;
use std::ops::BitXorAssign;

use bitvec::vec::BitVec;
use chrono::{DateTime, NaiveDateTime, Utc};
use diesel::dsl::sql;
use diesel::sql_types::{Binary, Bool, Integer, Nullable, Text, Timestamp};
use diesel::sqlite::Sqlite;
use diesel::upsert::excluded;
use diesel::{
    BelongingToDsl, ExpressionMethods, NullableExpressionMethods, OptionalExtension, QueryDsl,
    SelectableHelper, define_sql_function,
};
use diesel_async::{AsyncConnection, RunQueryDsl};
use futures::FutureExt;
use miette::{IntoDiagnostic, WrapErr, miette};
use serde_json;

use crate::asset_storage::AssetSpec;
use crate::location::Location;
use crate::state::interface::WorkflowRunStateSummary;
use crate::state::models::{
    NewNodeOutput, NewWorkflow, NewWorkflowRun, NewWorkflowRunInput, NodeOutput, NodeState,
    UpsertNodeState, UpsertWorkflowRun, Workflow, WorkflowRun, WorkflowRunAttempt,
    WorkflowRunInput,
};
use crate::workflow_state_query::NodeCopyFilter;

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
    metadata: &[u8],
) -> diesel::result::QueryResult<WorkflowRun> {
    use crate::state::schema::workflow_run_attempts::dsl as wra;
    use crate::state::schema::workflow_runs::dsl as wr;

    let workflow_run = diesel::insert_into(wr::workflow_runs)
        .values(run)
        .on_conflict_do_nothing()
        .returning(WorkflowRun::as_returning())
        .get_result(conn)
        .await?;

    // The current sqlite implementation always adds workflows to
    // the active set when they inserted into the database, so
    // set the started time to now.
    let now = chrono::Utc::now().naive_utc();
    diesel::insert_into(wra::workflow_run_attempts)
        .values((
            wra::workflow_run_id.eq(&workflow_run.id),
            wra::started_time.eq(Some(now)),
            wra::run_metadata.eq(jsonb(metadata)),
        ))
        .execute(conn)
        .await?;

    Ok(workflow_run)
}

define_sql_function!(
    /// Convert JSON bytes to `SQLite` JSONB.
    fn jsonb(value: Binary) -> Binary;
);

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

/// Update the persisted timing state for a workflow run attempt.
///
/// # Errors
///
/// Returns an error when the update fails.
pub async fn update_workflow_run(
    conn: &mut impl AsyncConnection<Backend = Sqlite>,
    run_id: &str,
    attempt: i32,
    workflow_update: UpsertWorkflowRun,
) -> miette::Result<bool> {
    use crate::state::schema::workflow_run_attempts::dsl as wra;

    let rows_affected = diesel::update(
        wra::workflow_run_attempts
            .filter(wra::workflow_run_id.eq(run_id))
            .filter(wra::attempt.eq(attempt)),
    )
    .set((
        wra::started_time.eq(coalesce_datetime(
            wra::started_time,
            workflow_update.started_time,
        )),
        wra::queued_time.eq(coalesce_datetime(
            wra::queued_time,
            workflow_update.queued_time,
        )),
        wra::complete_time.eq(coalesce_datetime(
            wra::complete_time,
            workflow_update.complete_time,
        )),
        wra::cancelled_time.eq(coalesce_datetime(
            wra::cancelled_time,
            workflow_update.cancelled_time,
        )),
        wra::error_time.eq(coalesce_datetime(
            wra::error_time,
            workflow_update.error_time,
        )),
    ))
    .execute(conn)
    .await
    .into_diagnostic()
    .wrap_err("Failed to update workflow run timing")?;

    Ok(rows_affected > 0)
}

/// Ensure a node-state row exists for the given run/location.
///
/// # Errors
///
/// Returns an error when the connection pool cannot be accessed or the insert
/// operation fails.
pub fn update_node_state<'a>(
    conn: &'a mut impl AsyncConnection<Backend = Sqlite>,
    run_id: &'a str,
    attempt: i32,
    mut node_updates: Vec<UpsertNodeState>,
    node_outputs: Vec<(Location, NewNodeOutput<'a>)>,
) -> impl Future<Output = miette::Result<bool>> + Send + 'a {
    use crate::state::schema::node_states::dsl as ns;

    conn.transaction(async move |conn| {
        merge_map_complete_fields(conn, run_id, attempt, &mut node_updates).await?;

        // TODO: We shouldn't need to loop if batch inserts for sqlite and diesel_async are patched.
        let mut rows_affected = 0;
        for node_update in node_updates {
            rows_affected += diesel::insert_into(ns::node_states)
                .values(node_update)
                .on_conflict((ns::run_id, ns::attempt, ns::node_location))
                .do_update()
                .set((
                    ns::name.eq(coalesce_text(ns::name, excluded(ns::name))),
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
                    ns::paused_time.eq(coalesce_datetime(
                        ns::paused_time,
                        excluded(ns::paused_time),
                    )),
                    ns::unpaused_time.eq(coalesce_datetime(
                        ns::unpaused_time,
                        excluded(ns::unpaused_time),
                    )),
                    ns::complete_time.eq(coalesce_datetime(
                        ns::complete_time,
                        excluded(ns::complete_time),
                    )),
                    ns::cancelled_time.eq(coalesce_datetime(
                        ns::cancelled_time,
                        excluded(ns::cancelled_time),
                    )),
                    ns::error_time.eq(coalesce_datetime(ns::error_time, excluded(ns::error_time))),
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
                    ns::handle.eq(coalesce_text(excluded(ns::handle), ns::handle)),
                    ns::execution_context.eq(coalesce_text(
                        excluded(ns::execution_context),
                        ns::execution_context,
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
    })
    .map(IntoDiagnostic::into_diagnostic)
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
        let handle = db_node.handle.clone();

        Ok(crate::state::interface::NodeState {
            name: db_node.name.clone(),
            scheduled_time: db_node.scheduled_time.map(utc_timestamp),
            queued_time: db_node.queued_time.map(utc_timestamp),
            paused_time: db_node.paused_time.map(utc_timestamp),
            unpaused_time: db_node.unpaused_time.map(utc_timestamp),
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
            handle,
            execution_context: db_node.execution_context.clone(),
        })
    } else {
        Ok(crate::state::interface::NodeState::default())
    }
}

/// Read the persisted node state for a workflow run for a specific node name.
///
/// # Errors
///
/// Returns an error when the connection pool cannot be accessed or the node state
/// lookup fails.
pub async fn read_node_states_by_name(
    conn: &mut impl AsyncConnection<Backend = Sqlite>,
    run_id: uuid::Uuid,
    attempt: u32,
    name: &str,
) -> miette::Result<HashMap<Location, crate::state::interface::NodeState>> {
    use crate::state::schema::node_states::dsl as ns;

    let mut states = HashMap::new();
    let attempt_i32 = i32::try_from(attempt)
        .into_diagnostic()
        .wrap_err_with(|| miette!("Attempt value {attempt} does not fit into i32"))?;
    let db_nodes = ns::node_states
        .filter(ns::run_id.eq(run_id.to_string()))
        .filter(ns::attempt.eq(attempt_i32))
        .filter(ns::name.eq(name))
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
        let handle = db_node.handle.clone();

        states.insert(
            db_node.node_location,
            crate::state::interface::NodeState {
                name: db_node.name.clone(),
                scheduled_time: db_node.scheduled_time.map(utc_timestamp),
                queued_time: db_node.queued_time.map(utc_timestamp),
                paused_time: db_node.paused_time.map(utc_timestamp),
                unpaused_time: db_node.unpaused_time.map(utc_timestamp),
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
                handle,
                execution_context: db_node.execution_context.clone(),
            },
        );
    }

    Ok(states)
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
        let handle = db_node.handle.clone();

        states.insert(
            db_node.node_location,
            crate::state::interface::NodeState {
                name: db_node.name.clone(),
                scheduled_time: db_node.scheduled_time.map(utc_timestamp),
                queued_time: db_node.queued_time.map(utc_timestamp),
                paused_time: db_node.paused_time.map(utc_timestamp),
                unpaused_time: db_node.unpaused_time.map(utc_timestamp),
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
                handle,
                execution_context: db_node.execution_context.clone(),
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

/// Read the persisted workflow inputs for the workflow at a location.
///
/// # Errors
///
/// Returns an error when the connection pool cannot be accessed or the workflow run
/// input lookup fails.
pub async fn read_workflow_run_input_from_location(
    conn: &mut impl AsyncConnection<Backend = Sqlite>,
    workflow_run_id: &str,
    attempt: i32,
    location: &str,
    name: &str,
) -> miette::Result<AssetSpec> {
    use crate::state::schema::workflow_run_inputs::dsl as wri;

    let input = wri::workflow_run_inputs
        .select(WorkflowRunInput::as_select())
        .filter(wri::workflow_run_id.eq(workflow_run_id))
        .filter(wri::attempt.eq(attempt))
        .filter(wri::location.eq(location))
        .filter(wri::name.eq(name))
        .first(conn)
        .await
        .into_diagnostic()?;

    Ok(AssetSpec {
        asset_key: input.asset_key.parse().into_diagnostic()?,
        kind: input.asset_kind.parse()?,
        storage_name: input.storage_name,
    })
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

/// List all workflow runs with summary information for display in the visualizer.
///
/// # Errors
///
/// Returns an error when the connection pool cannot be accessed or the queries fail.
pub async fn list_workflow_run_summaries(
    conn: &mut impl AsyncConnection<Backend = Sqlite>,
) -> miette::Result<Vec<WorkflowRunStateSummary>> {
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

        let paused_locations: Vec<Location> = ns::node_states
            .select(ns::node_location)
            .filter(ns::run_id.eq(&run.id))
            .filter(ns::attempt.eq(run_attempt.attempt))
            .filter(ns::paused_time.is_not_null())
            .filter(ns::unpaused_time.is_null())
            .get_results(conn)
            .await
            .into_diagnostic()
            .wrap_err_with(|| miette!("Failed to list paused nodes for run {}", run.id))?;

        let errored_locations: Vec<Location> = ns::node_states
            .select(ns::node_location)
            .filter(ns::run_id.eq(&run.id))
            .filter(ns::attempt.eq(run_attempt.attempt))
            .filter(ns::error_time.is_not_null())
            .get_results(conn)
            .await
            .into_diagnostic()
            .wrap_err_with(|| miette!("Failed to list errored nodes for run {}", run.id))?;

        summaries.push(WorkflowRunStateSummary {
            run_id,
            attempt,
            workflow_id,
            name: workflow_name,
            started_time: run_attempt.started_time.map(utc_timestamp),
            queued_time: run_attempt.queued_time.map(utc_timestamp),
            complete_time: run_attempt.complete_time.map(utc_timestamp),
            cancelled_time: run_attempt.cancelled_time.map(utc_timestamp),
            error_time: run_attempt.error_time.map(utc_timestamp),
            paused_locations,
            errored_locations,
        });
    }

    Ok(summaries)
}

/// Retrieve the `WorkflowRunStateSummary` for a particular workflow run attempt.
///
/// # Errors
///
/// Returns an error when the connection pool cannot be accessed or the queries fail.
pub async fn get_workflow_run_summary(
    conn: &mut impl AsyncConnection<Backend = Sqlite>,
    run_id: uuid::Uuid,
    attempt: u32,
) -> miette::Result<WorkflowRunStateSummary> {
    use crate::state::schema::node_states::dsl as ns;
    use crate::state::schema::workflow_run_attempts::dsl as wra;
    use crate::state::schema::workflow_runs::dsl as wr;
    use crate::state::schema::workflows::dsl as wf;

    let attempt_i32 = i32::try_from(attempt)
        .into_diagnostic()
        .wrap_err_with(|| miette!("Attempt value {attempt} does not fit into i32"))?;

    let run_id_str = run_id.to_string();

    let (run, run_attempt, name): (WorkflowRun, WorkflowRunAttempt, Option<String>) =
        wr::workflow_runs
            .inner_join(wra::workflow_run_attempts)
            .left_join(wf::workflows)
            .select((
                WorkflowRun::as_select(),
                WorkflowRunAttempt::as_select(),
                wf::name.nullable(),
            ))
            .filter(wr::id.eq(&run_id_str))
            .filter(wra::attempt.eq(attempt_i32))
            .get_result(conn)
            .await
            .into_diagnostic()
            .wrap_err("Failed to list workflow runs")?;

    let workflow_id: uuid::Uuid = run
        .workflow_id
        .parse()
        .into_diagnostic()
        .wrap_err_with(|| miette!("Invalid workflow UUID: {}", run.workflow_id))?;

    let paused_locations: Vec<Location> = ns::node_states
        .select(ns::node_location)
        .filter(ns::run_id.eq(&run.id))
        .filter(ns::attempt.eq(run_attempt.attempt))
        .filter(ns::paused_time.is_not_null())
        .filter(ns::unpaused_time.is_null())
        .get_results(conn)
        .await
        .into_diagnostic()
        .wrap_err_with(|| miette!("Failed to list paused nodes for run {}", run.id))?;

    let errored_locations: Vec<Location> = ns::node_states
        .select(ns::node_location)
        .filter(ns::run_id.eq(&run_id_str))
        .filter(ns::attempt.eq(attempt_i32))
        .filter(ns::error_time.is_not_null())
        .get_results(conn)
        .await
        .into_diagnostic()
        .wrap_err_with(|| miette!("Failed to list errored nodes for run {}", run.id))?;

    Ok(WorkflowRunStateSummary {
        run_id,
        attempt,
        workflow_id,
        name,
        started_time: run_attempt.started_time.map(utc_timestamp),
        queued_time: run_attempt.queued_time.map(utc_timestamp),
        complete_time: run_attempt.complete_time.map(utc_timestamp),
        cancelled_time: run_attempt.cancelled_time.map(utc_timestamp),
        error_time: run_attempt.error_time.map(utc_timestamp),
        paused_locations,
        errored_locations,
    })
}

/// List all workflow run attempts that have not yet reached a terminal state.
/// Used during crash recovery to identify interrupted runs.
///
/// # Errors
///
/// Returns an error when the connection pool cannot be accessed or the query fails.
pub async fn list_active_runs(
    conn: &mut impl AsyncConnection<Backend = Sqlite>,
) -> miette::Result<Vec<(uuid::Uuid, u32)>> {
    use crate::state::schema::workflow_run_attempts::dsl as wra;

    let runs: Vec<(String, i32)> = wra::workflow_run_attempts
        .select((wra::workflow_run_id, wra::attempt))
        .filter(wra::complete_time.is_null())
        .filter(wra::cancelled_time.is_null())
        .filter(wra::error_time.is_null())
        .get_results(conn)
        .await
        .into_diagnostic()
        .wrap_err("Failed to list interrupted workflow runs")?;

    runs.into_iter()
        .map(|(run_id_str, attempt_i32)| {
            let run_id = run_id_str.parse().into_diagnostic()?;
            let attempt = u32::try_from(attempt_i32).into_diagnostic()?;
            Ok((run_id, attempt))
        })
        .collect()
}

/// Return the highest existing attempt number for a given `run_id`, if any.
///
/// # Errors
///
/// Returns an error when the connection pool cannot be accessed or the query fails.
pub async fn max_attempt(
    conn: &mut impl AsyncConnection<Backend = Sqlite>,
    run_id: &str,
) -> miette::Result<Option<i32>> {
    use crate::state::schema::workflow_run_attempts::dsl as wra;

    wra::workflow_run_attempts
        .filter(wra::workflow_run_id.eq(run_id))
        .select(diesel::dsl::max(wra::attempt))
        .first(conn)
        .await
        .into_diagnostic()
        .wrap_err("Failed to query max attempt")
}

/// Insert a new `workflow_run_attempts` row for an existing run with an explicit
/// attempt number.
///
/// # Errors
///
/// Returns an error when the insert fails.
pub async fn insert_workflow_run_attempt(
    conn: &mut impl AsyncConnection<Backend = Sqlite>,
    run_id: &str,
    attempt: i32,
) -> miette::Result<()> {
    use crate::state::schema::workflow_run_attempts::dsl as wra;

    let metadata = wra::workflow_run_attempts
        .filter(wra::workflow_run_id.eq(run_id))
        .filter(wra::attempt.eq(attempt - 1))
        .select(wra::run_metadata)
        .first::<Vec<u8>>(conn)
        .await
        .into_diagnostic()?;
    let now = chrono::Utc::now().naive_utc();
    diesel::insert_into(wra::workflow_run_attempts)
        .values((
            wra::workflow_run_id.eq(run_id),
            wra::attempt.eq(attempt),
            wra::run_metadata.eq(metadata),
            wra::started_time.eq(Some(now)),
        ))
        .execute(conn)
        .await
        .into_diagnostic()
        .wrap_err("Failed to insert new workflow run attempt")?;
    Ok(())
}

/// Copy node states (and their outputs) from `source_attempt` to `dest_attempt` of
/// the same `run_id`.
///
/// # Errors
///
/// Returns an error when the connection pool cannot be accessed or a query fails.
///
/// # Panics
///
/// Will panic if the provided `reset_loop_index` in `node_copy_filter` is greater than `i32_MAX`.
pub fn copy_node_states<'a>(
    conn: &'a mut impl AsyncConnection<Backend = Sqlite>,
    run_id: &'a str,
    source_attempt: i32,
    dest_attempt: i32,
    node_copy_filter: &'a NodeCopyFilter,
) -> impl Future<Output = miette::Result<()>> + Send + 'a {
    use crate::state::schema::node_states::dsl as ns;

    conn.transaction(async move |conn| {
        let rows: Vec<NodeState> = ns::node_states
            .filter(ns::run_id.eq(&run_id))
            .filter(ns::attempt.eq(source_attempt))
            .get_results(conn)
            .await?;

        for row in rows {
            if node_copy_filter.clear_roots.contains(&row.node_location)
                || row
                    .node_location
                    .node_ancestors()
                    .any(|ancestor| node_copy_filter.clear_roots.contains(&ancestor))
            {
                continue;
            }
            let is_truncated = node_copy_filter.truncate_exact.contains(&row.node_location);
            let loop_index = node_copy_filter
                .reset_loop_index
                .get(&row.node_location)
                .map(|index| {
                    TryInto::<i32>::try_into(*index).expect("Failed to convert to signed integer")
                })
                .or(row.loop_index);
            let map_completed = node_copy_filter
                .reset_map_elems
                .get(&row.node_location)
                .and_then(|bit_mask| {
                    row.map_completed.clone().map(|existing| {
                        let mut new = BitVec::<u8, bitvec::order::Lsb0>::from_vec(existing);
                        new.bitxor_assign(bit_mask);
                        new.into_vec()
                    })
                });
            let new_row = if is_truncated {
                // Reset all timestamps
                UpsertNodeState {
                    name: row.name.clone(),
                    run_id: run_id.to_string(),
                    attempt: dest_attempt,
                    node_location: row.node_location.clone(),
                    cond: row.cond, // retained to preserve conditional state even when truncated
                    loop_index,
                    map_size: row.map_size,
                    map_completed,
                    execution_context: row.execution_context.clone(),
                    ..Default::default()
                }
            } else {
                UpsertNodeState {
                    name: row.name.clone(),
                    run_id: run_id.to_string(),
                    attempt: dest_attempt,
                    node_location: row.node_location.clone(),
                    scheduled_time: row.scheduled_time,
                    paused_time: row.paused_time,
                    unpaused_time: row.unpaused_time,
                    queued_time: row.queued_time,
                    running_time: row.running_time,
                    complete_time: row.complete_time,
                    cancelled_time: row.cancelled_time,
                    error_time: row.error_time,
                    cond: row.cond,
                    loop_index,
                    map_size: row.map_size,
                    map_completed,
                    handle: row.handle.clone(),
                    error: row.error.clone(),
                    error_detail: row.error_detail.clone(),
                    execution_context: row.execution_context.clone(),
                }
            };

            let new_id: i32 = diesel::insert_into(ns::node_states)
                .values(new_row)
                .returning(ns::id)
                .get_result(conn)
                .await?;
            // copy over node outputs to the new node state
            if !is_truncated {
                copy_outputs(conn, row.id, new_id).await?;
            }

            copy_inputs(
                conn,
                run_id,
                source_attempt,
                dest_attempt,
                &row.node_location,
            )
            .await?;
        }

        Ok::<_, diesel::result::Error>(())
    })
    .map(|res| {
        res.into_diagnostic()
            .wrap_err("Failed to copy node states to new attempt")
    })
}

async fn copy_outputs(
    conn: &mut impl AsyncConnection<Backend = Sqlite>,
    source_id: i32,
    new_id: i32,
) -> Result<(), diesel::result::Error> {
    use crate::state::schema::node_outputs::dsl as no;

    let previous_outputs = no::node_outputs
        .filter(no::node_state_id.eq(source_id))
        .select((
            // I wish there was a better way to do this
            sql::<Integer>(&new_id.to_string()),
            no::name,
            no::asset_kind,
            no::storage_name,
            no::asset_key,
        ));
    diesel::insert_into(no::node_outputs)
        .values(previous_outputs)
        .into_columns((
            no::node_state_id,
            no::name,
            no::asset_kind,
            no::storage_name,
            no::asset_key,
        ))
        .execute(conn)
        .await?;
    Ok(())
}

/// Copy workflow for a workflow run from one attempt to another
/// at a specific location.
///
/// # Errors
///
/// Will return Err if the query fails.
pub async fn copy_inputs(
    conn: &mut impl AsyncConnection<Backend = Sqlite>,
    run_id: &str,
    source_attempt: i32,
    dest_attempt: i32,
    location: &Location,
) -> Result<(), diesel::result::Error> {
    use crate::state::schema::workflow_run_inputs::dsl as wri;

    let previous_inputs = wri::workflow_run_inputs
        .filter(wri::workflow_run_id.eq(run_id))
        .filter(wri::attempt.eq(source_attempt))
        .filter(wri::location.eq(&location))
        .select((
            wri::workflow_run_id,
            // I wish there was a better way to do this
            sql::<Integer>(&dest_attempt.to_string()),
            wri::location,
            wri::name,
            wri::asset_kind,
            wri::storage_name,
            wri::asset_key,
        ));
    diesel::insert_into(wri::workflow_run_inputs)
        .values(previous_inputs)
        .into_columns((
            wri::workflow_run_id,
            wri::attempt,
            wri::location,
            wri::name,
            wri::asset_kind,
            wri::storage_name,
            wri::asset_key,
        ))
        .execute(conn)
        .await?;
    Ok(())
}
