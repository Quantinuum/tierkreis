/*!
This module defines the queries for reading the workflow state from the `SQlite` database.
*/
use std::collections::HashMap;
use std::hash::BuildHasher;

use chrono::{DateTime, NaiveDateTime, Utc};
use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool};
use miette::miette;

use crate::location::Location;
use crate::state::models::{NodeState, UpsertNodeState, Workflow, WorkflowRun};

/// [`RunAttemptState`] is the full state of a run.
#[derive(Debug, Default)]
#[allow(missing_docs)]
pub struct RunAttemptState {
    pub nodes: HashMap<Location, crate::state::interface::NodeState>,
    pub metadata: HashMap<String, String>,
}

fn utc_timestamp(ts: NaiveDateTime) -> DateTime<Utc> {
    DateTime::<Utc>::from_naive_utc_and_offset(ts, Utc)
}

/// Load the run-attempt state for a workflow run if it exists.
///
/// # Errors
///
/// If the run does not exists.
/// Returns an error when the connection pool cannot be accessed.
pub fn read_workflowrun(
    run_id: uuid::Uuid,
    attempt: u32,
    connection: &Pool<ConnectionManager<SqliteConnection>>,
) -> miette::Result<WorkflowRun> {
    use crate::state::schema::workflow_runs::dsl as wr;

    let mut conn = connection
        .get()
        .map_err(|err| miette!("Failed to get SQLite connection from pool: {err}"))?;

    let attempt_i32 = i32::try_from(attempt)
        .map_err(|_| miette!("Attempt value {attempt} does not fit into i32"))?;

    wr::workflow_runs
        .filter(wr::id.eq(run_id.to_string()))
        .filter(wr::attempt.eq(attempt_i32))
        .order(wr::started_time.asc())
        .select(WorkflowRun::as_select())
        .first::<WorkflowRun>(&mut conn)
        .map_err(|err| miette!("Failed to query workflow run: {err}"))
}

/// Insert a workflow run row.
///
/// # Errors
///
/// Returns an error when the insert fails.
pub fn insert_workflow_run(
    run: &WorkflowRun,
    connection: &Pool<ConnectionManager<SqliteConnection>>,
) -> miette::Result<()> {
    use crate::state::schema::workflow_runs::dsl as wr;

    let mut conn = connection
        .get()
        .map_err(|err| miette!("Failed to get SQLite connection from pool: {err}"))?;

    diesel::insert_into(wr::workflow_runs)
        .values(run)
        .on_conflict((wr::id, wr::attempt))
        .do_nothing()
        .execute(&mut conn)
        .map_err(|err| {
            miette!(
                "Failed to insert workflow run row for run {} attempt {}: {err}",
                run.id,
                run.attempt
            )
        })?;

    Ok(())
}

/// Insert a default workflow run row for a given run ID and attempt.
/// Also insert a default workflow row if it does not already exist.
///
/// # Errors
///
/// Returns an error when the connection pool cannot be accessed or the insert
/// fails.
pub fn insert_default_workflowrun(
    run_id: uuid::Uuid,
    attempt: u32,
    connection: &Pool<ConnectionManager<SqliteConnection>>,
) -> miette::Result<WorkflowRun> {
    use crate::state::schema::workflows::dsl as wf;

    let mut conn = connection
        .get()
        .map_err(|err| miette!("Failed to get SQLite connection from pool: {err}"))?;

    let attempt_i32 = i32::try_from(attempt)
        .map_err(|_| miette!("Attempt value {attempt} does not fit into i32"))?;

    let workflow_id = uuid::Uuid::nil().to_string();
    let workflow = Workflow {
        id: workflow_id.clone(),
        name: None,
        created_time: Some(Utc::now().naive_utc()),
    };

    diesel::insert_or_ignore_into(wf::workflows)
        .values(&workflow)
        .execute(&mut conn)
        .map_err(|err| miette!("Failed to insert workflow row for run {}: {err}", run_id))?;
    let run = WorkflowRun {
        id: run_id.to_string(),
        attempt: attempt_i32,
        workflow_id: workflow_id.clone(),
        run_metadata: br"{}".to_vec(),
        status: None,
        started_time: None,
    };

    insert_workflow_run(&run, connection)?;
    Ok(run)
}

/// Upsert the current node state for a workflow run at a given location.
///
/// # Errors
///
/// Returns an error when the connection pool cannot be accessed or the node state
/// upsert fails.
pub fn update_node_state(
    state: &mut crate::state::interface::NodeState,
    loc: &Location,
    run_id: uuid::Uuid,
    attempt: u32,
    connection: &Pool<ConnectionManager<SqliteConnection>>,
) -> miette::Result<()> {
    use crate::state::schema::node_states::dsl as ns;

    let mut conn = connection
        .get()
        .map_err(|err| miette!("Failed to get SQLite connection from pool: {err}"))?;

    let attempt_i32 = i32::try_from(attempt)
        .map_err(|_| miette!("Attempt value {attempt} does not fit into i32"))?;
    let row = UpsertNodeState {
        run_id: run_id.to_string(),
        attempt: attempt_i32,
        node_location: loc.clone(),
        scheduled_time: state.scheduled_time.map(|t| t.naive_utc()),
        queued_time: state.queued_time.map(|t| t.naive_utc()),
        running_time: state.running_time.map(|t| t.naive_utc()),
        complete_time: state.complete_time.map(|t| t.naive_utc()),
        cancelled_time: state.cancelled_time.map(|t| t.naive_utc()),
        error_time: state.error_time.map(|t| t.naive_utc()),
        error: state.error.clone(),
        error_detail: state.error_detail.clone(),
    };

    diesel::insert_into(ns::node_states)
        .values(&row)
        .on_conflict((ns::run_id, ns::attempt, ns::node_location))
        .do_update()
        .set(&row)
        .execute(&mut conn)
        .map_err(|err| {
            miette!(
                "Failed to upsert node state for run {run_id} attempt {attempt} location {:?}: {err}",
                loc
            )
        })?;

    Ok(())
}

/// Read the persisted node state for a workflow run at a given location.
///
/// # Errors
///
/// Returns an error when the connection pool cannot be accessed or the node state
/// lookup fails.
pub fn read_node_state(
    run_id: uuid::Uuid,
    attempt: u32,
    loc: &Location,
    connection: &Pool<ConnectionManager<SqliteConnection>>,
) -> miette::Result<crate::state::interface::NodeState> {
    use crate::state::schema::node_states::dsl as ns;

    let mut conn = connection
        .get()
        .map_err(|err| miette!("Failed to get SQLite connection from pool: {err}"))?;

    let attempt_i32 = i32::try_from(attempt)
        .map_err(|_| miette!("Attempt value {attempt} does not fit into i32"))?;
    let db_node = ns::node_states
        .filter(ns::run_id.eq(run_id.to_string()))
        .filter(ns::attempt.eq(attempt_i32))
        .filter(ns::node_location.eq(loc.clone()))
        .first::<NodeState>(&mut conn)
        .optional()
        .map_err(|err| {
            miette!(
                "Failed to query node state for run {} attempt {} location {:?}: {err}",
                run_id,
                attempt,
                loc
            )
        })?;

    if let Some(db_node) = db_node {
        Ok(crate::state::interface::NodeState {
            scheduled_time: db_node.scheduled_time.map(utc_timestamp),
            queued_time: db_node.queued_time.map(utc_timestamp),
            running_time: db_node.running_time.map(utc_timestamp),
            complete_time: db_node.complete_time.map(utc_timestamp),
            cancelled_time: db_node.cancelled_time.map(utc_timestamp),
            error_time: db_node.error_time.map(utc_timestamp),
            error: db_node.error.clone(),
            error_detail: db_node.error_detail.clone(),
            ..Default::default()
        })
    } else {
        Ok(crate::state::interface::NodeState::default())
    }
}

/// Merge additional metadata into the persisted run metadata for a workflow run.
///
/// # Errors
///
/// Returns an error when the connection pool cannot be accessed, the run lookup
/// fails, metadata JSON cannot be parsed or serialized, or the update fails.
pub fn add_run_metadata<S: BuildHasher>(
    run_id: uuid::Uuid,
    attempt: u32,
    new_metadata: HashMap<String, String, S>,
    connection: &Pool<ConnectionManager<SqliteConnection>>,
) -> miette::Result<()> {
    use crate::state::schema::workflow_runs::dsl as wr;

    let mut conn = connection
        .get()
        .map_err(|err| miette!("Failed to get SQLite connection from pool: {err}"))?;

    let attempt_i32 = i32::try_from(attempt)
        .map_err(|_| miette!("Attempt value {attempt} does not fit into i32"))?;

    let run_id_str = run_id.to_string();

    let run = wr::workflow_runs
        .filter(wr::id.eq(run_id_str.clone()))
        .filter(wr::attempt.eq(attempt_i32))
        .first::<WorkflowRun>(&mut conn);
    let run = match run {
        Ok(run) => run,
        Err(diesel::result::Error::NotFound) => {
            insert_default_workflowrun(run_id, attempt, connection)
                .map_err(|err| miette!("Failed to insert default run for metadata update: {err}"))?
        }
        Err(err) => {
            return Err(miette!(
                "Failed to query workflow run for metadata update: {err}"
            ));
        }
    };

    let mut metadata = serde_json::from_slice::<HashMap<String, String>>(&run.run_metadata)
        .map_err(|err| {
            miette!(
                "Failed to parse existing run metadata JSON for run {}: {err}",
                run.id
            )
        })?;

    metadata.extend(new_metadata);

    let updated_metadata_json = serde_json::to_vec(&metadata).map_err(|err| {
        miette!(
            "Failed to serialize updated metadata to JSON for run {}: {err}",
            run.id
        )
    })?;

    diesel::update(
        wr::workflow_runs
            .filter(wr::id.eq(run_id_str))
            .filter(wr::attempt.eq(attempt_i32)),
    )
    .set(wr::run_metadata.eq(updated_metadata_json))
    .execute(&mut conn)
    .map_err(|err| {
        miette!(
            "Failed to update workflow run metadata for run {} attempt {}: {err}",
            run_id,
            attempt
        )
    })?;

    Ok(())
}

/// Read the persisted metadata for a workflow run.
///
/// # Errors
///
/// Returns an error when the connection pool cannot be accessed, the run lookup
/// fails, or the metadata JSON cannot be parsed.
pub fn read_run_metadata(
    run_id: uuid::Uuid,
    attempt: u32,
    connection: &Pool<ConnectionManager<SqliteConnection>>,
) -> miette::Result<HashMap<String, String>> {
    use crate::state::schema::workflow_runs::dsl as wr;

    let mut conn = connection
        .get()
        .map_err(|err| miette!("Failed to get SQLite connection from pool: {err}"))?;

    let attempt_i32 = i32::try_from(attempt)
        .map_err(|_| miette!("Attempt value {attempt} does not fit into i32"))?;

    let run_id_str = run_id.to_string();

    let run = wr::workflow_runs
        .filter(wr::id.eq(run_id_str.clone()))
        .filter(wr::attempt.eq(attempt_i32))
        .first::<WorkflowRun>(&mut conn)
        .map_err(|err| miette!("Failed to query workflow run for metadata update: {err}"))?;

    let metadata =
        serde_json::from_slice::<HashMap<String, String>>(&run.run_metadata).map_err(|err| {
            miette!(
                "Failed to parse existing run metadata JSON for run {}: {err}",
                run.id
            )
        })?;

    Ok(metadata)
}
