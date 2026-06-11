/*!
This module defines the queries for reading the workflow state from the `SQlite` database.
*/
use std::collections::HashMap;
use std::hash::BuildHasher;

use bitvec::vec::BitVec;
use chrono::{DateTime, NaiveDateTime, Utc};
use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool};
use miette::miette;

use crate::location::Location;
use crate::state::models::{NodeState, UpsertNodeState, Workflow, WorkflowRun};

fn utc_timestamp(ts: NaiveDateTime) -> DateTime<Utc> {
    DateTime::<Utc>::from_naive_utc_and_offset(ts, Utc)
}

fn encode_map_completed(map_completed: &BitVec) -> miette::Result<Vec<u8>> {
    serde_json::to_vec(map_completed)
        .map_err(|err| miette!("Failed to serialize map_completed: {err}"))
}

fn decode_map_completed(encoded: &[u8]) -> miette::Result<BitVec> {
    serde_json::from_slice(encoded)
        .map_err(|err| miette!("Failed to deserialize map_completed: {err}"))
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

    // This workflow should exist already, this is just to have a valid insert
    // In the future this should not be necessary. Nil to make it clear it is not a real workflow.
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

/// Ensure a node-state row exists for the given run/location.
///
/// # Errors
///
/// Returns an error when the connection pool cannot be accessed or the insert
/// operation fails.
pub fn insert_default_node_state(
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
        scheduled_time: None,
        queued_time: None,
        running_time: None,
        complete_time: None,
        cancelled_time: None,
        error_time: None,
        cond: None,
        loop_index: None,
        map_completed: None,
        error: None,
        error_detail: None,
    };

    diesel::insert_into(ns::node_states)
        .values(&row)
        .on_conflict((ns::run_id, ns::attempt, ns::node_location))
        .do_nothing()
        .execute(&mut conn)
        .map_err(|err| {
            miette!(
                "Failed to ensure node state row for run {run_id} attempt {attempt} location {:?}: {err}",
                loc
            )
        })?;

    Ok(())
}

macro_rules! define_set_time_if_none {
    ($fn_name:ident, $field:ident, $label:literal) => {
        #[doc = "Set `"]
        #[doc = stringify!($field)]
        #[doc = "` iff it is currently unset. \n\n# Errors\n\nReturns an error if the connection pool cannot be accessed or the update fails."]

        pub fn $fn_name(
            loc: &Location,
            run_id: uuid::Uuid,
            attempt: u32,
            connection: &Pool<ConnectionManager<SqliteConnection>>,
        ) -> miette::Result<bool> {
            use crate::state::schema::node_states::dsl as ns;

            let mut conn = connection
                .get()
                .map_err(|err| miette!("Failed to get SQLite connection from pool: {err}"))?;
            let attempt_i32 = i32::try_from(attempt)
                .map_err(|_| miette!("Attempt value {attempt} does not fit into i32"))?;
            let now = Utc::now().naive_utc();

            let changed = diesel::update(
                ns::node_states
                    .filter(ns::run_id.eq(run_id.to_string()))
                    .filter(ns::attempt.eq(attempt_i32))
                    .filter(ns::node_location.eq(loc.clone()))
                    .filter(ns::$field.is_null()),
            )
            .set(ns::$field.eq(now))
            .execute(&mut conn)
            .map_err(|err| {
                miette!(
                    "Failed to set {} time for run {run_id} attempt {attempt} location {:?}: {err}",
                    $label,
                    loc
                )
            })?;

            Ok(changed > 0)
        }
    };
}

define_set_time_if_none!(set_scheduled_if_none, scheduled_time, "scheduled");
define_set_time_if_none!(set_queued_if_none, queued_time, "queued");
define_set_time_if_none!(set_running_if_none, running_time, "running");
define_set_time_if_none!(set_complete_if_none, complete_time, "complete");
define_set_time_if_none!(set_cancelled_if_none, cancelled_time, "cancelled");

/// Set error fields iff `error_time` is currently unset.
///
/// # Errors
///
/// Returns an error if the connection pool cannot be accessed or the update fails.
pub fn set_error_if_none(
    loc: &Location,
    run_id: uuid::Uuid,
    attempt: u32,
    error: String,
    detail: Option<String>,
    connection: &Pool<ConnectionManager<SqliteConnection>>,
) -> miette::Result<bool> {
    use crate::state::schema::node_states::dsl as ns;

    let mut conn = connection
        .get()
        .map_err(|err| miette!("Failed to get SQLite connection from pool: {err}"))?;
    let attempt_i32 = i32::try_from(attempt)
        .map_err(|_| miette!("Attempt value {attempt} does not fit into i32"))?;
    let now = Utc::now().naive_utc();

    let changed = diesel::update(
        ns::node_states
            .filter(ns::run_id.eq(run_id.to_string()))
            .filter(ns::attempt.eq(attempt_i32))
            .filter(ns::node_location.eq(loc.clone()))
            .filter(ns::error_time.is_null()),
    )
    .set((
        ns::error_time.eq(now),
        ns::error.eq(error),
        ns::error_detail.eq(detail),
    ))
    .execute(&mut conn)
    .map_err(|err| {
        miette!(
            "Failed to set error state for run {run_id} attempt {attempt} location {:?}: {err}",
            loc
        )
    })?;

    Ok(changed > 0)
}

/// Set node cond state iff it is currently unset.
///
/// # Errors
///
/// Returns an error if the connection pool cannot be accessed or the update fails.
pub fn set_cond_if_none(
    loc: &Location,
    run_id: uuid::Uuid,
    attempt: u32,
    cond: bool,
    connection: &Pool<ConnectionManager<SqliteConnection>>,
) -> miette::Result<bool> {
    use crate::state::schema::node_states::dsl as ns;

    let mut db_conn = connection
        .get()
        .map_err(|err| miette!("Failed to get SQLite connection from pool: {err}"))?;
    let attempt_i32 = i32::try_from(attempt)
        .map_err(|_| miette!("Attempt value {attempt} does not fit into i32"))?;

    let changed = diesel::update(
        ns::node_states
            .filter(ns::run_id.eq(run_id.to_string()))
            .filter(ns::attempt.eq(attempt_i32))
            .filter(ns::node_location.eq(loc.clone()))
            .filter(ns::cond.is_null()),
    )
    .set(ns::cond.eq(cond))
    .execute(&mut db_conn)
    .map_err(|err| {
        miette!(
            "Failed to set cond state for run {run_id} attempt {attempt} location {:?}: {err}",
            loc
        )
    })?;

    Ok(changed > 0)
}

/// Set node loop index iff it differs from the current value.
///
/// # Errors
///
/// Returns an error if the connection pool cannot be accessed, conversion fails, or the update fails.
pub fn set_loop_index(
    loc: &Location,
    run_id: uuid::Uuid,
    attempt: u32,
    loop_index: u32,
    connection: &Pool<ConnectionManager<SqliteConnection>>,
) -> miette::Result<bool> {
    use crate::state::schema::node_states::dsl as ns;

    let mut conn = connection
        .get()
        .map_err(|err| miette!("Failed to get SQLite connection from pool: {err}"))?;
    let attempt_i32 = i32::try_from(attempt)
        .map_err(|_| miette!("Attempt value {attempt} does not fit into i32"))?;
    let loop_index_i32 = i32::try_from(loop_index)
        .map_err(|_| miette!("Loop index value {loop_index} does not fit into i32"))?;

    let changed = diesel::update(
        ns::node_states
            .filter(ns::run_id.eq(run_id.to_string()))
            .filter(ns::attempt.eq(attempt_i32))
            .filter(ns::node_location.eq(loc.clone()))
            .filter(
                ns::loop_index
                    .ne(loop_index_i32)
                    .or(ns::loop_index.is_null()),
            ),
    )
    .set(ns::loop_index.eq(loop_index_i32))
    .execute(&mut conn)
    .map_err(|err| {
        miette!(
            "Failed to set loop index for run {run_id} attempt {attempt} location {:?}: {err}",
            loc
        )
    })?;

    Ok(changed > 0)
}

/// Initialize map completion state if it is currently unset.
///
/// # Errors
///
/// Returns an error if the connection pool cannot be accessed or the update fails.
pub fn set_map_started_if_none(
    loc: &Location,
    run_id: uuid::Uuid,
    attempt: u32,
    size: u32,
    connection: &Pool<ConnectionManager<SqliteConnection>>,
) -> miette::Result<bool> {
    use crate::state::schema::node_states::dsl as ns;

    let mut conn = connection
        .get()
        .map_err(|err| miette!("Failed to get SQLite connection from pool: {err}"))?;
    let attempt_i32 = i32::try_from(attempt)
        .map_err(|_| miette!("Attempt value {attempt} does not fit into i32"))?;
    let encoded = encode_map_completed(&BitVec::repeat(false, size as usize))?;

    let changed = diesel::update(
        ns::node_states
            .filter(ns::run_id.eq(run_id.to_string()))
            .filter(ns::attempt.eq(attempt_i32))
            .filter(ns::node_location.eq(loc.clone()))
            .filter(ns::map_completed.is_null()),
    )
    .set(ns::map_completed.eq(encoded))
    .execute(&mut conn)
    .map_err(|err| {
        miette!(
            "Failed to initialize map completion for run {run_id} attempt {attempt} location {:?}: {err}",
            loc
        )
    })?;

    Ok(changed > 0)
}

/// Mark a map element as complete.
///
/// # Errors
///
/// Returns an error if the connection pool cannot be accessed, map state is invalid, or updates fail.
pub fn set_map_elem_complete(
    loc: &Location,
    run_id: uuid::Uuid,
    attempt: u32,
    index: u32,
    connection: &Pool<ConnectionManager<SqliteConnection>>,
) -> miette::Result<bool> {
    use crate::state::schema::node_states::dsl as ns;

    let mut conn = connection
        .get()
        .map_err(|err| miette!("Failed to get SQLite connection from pool: {err}"))?;
    let attempt_i32 = i32::try_from(attempt)
        .map_err(|_| miette!("Attempt value {attempt} does not fit into i32"))?;

    let map_completed = ns::node_states
        .filter(ns::run_id.eq(run_id.to_string()))
        .filter(ns::attempt.eq(attempt_i32))
        .filter(ns::node_location.eq(loc.clone()))
        .select(ns::map_completed)
        .first::<Option<Vec<u8>>>(&mut conn)
        .optional()
        .map_err(|err| {
            miette!(
                "Failed to load map completion for run {run_id} attempt {attempt} location {:?}: {err}",
                loc
            )
        })?
        .flatten();

    let Some(encoded) = map_completed else {
        return Ok(false);
    };

    let mut map_completed = decode_map_completed(&encoded)?;
    let index_usize = usize::try_from(index)
        .map_err(|_| miette!("Map completion index {index} does not fit into usize"))?;
    if index_usize >= map_completed.len() {
        return Err(miette!(
            "Map completion index {index} out of bounds for map of size {}",
            map_completed.len()
        ));
    }
    if map_completed[index_usize] {
        return Ok(false);
    }

    map_completed.set(index_usize, true);
    let updated = encode_map_completed(&map_completed)?;
    let changed = diesel::update(
        ns::node_states
            .filter(ns::run_id.eq(run_id.to_string()))
            .filter(ns::attempt.eq(attempt_i32))
            .filter(ns::node_location.eq(loc.clone())),
    )
    .set(ns::map_completed.eq(updated))
    .execute(&mut conn)
    .map_err(|err| {
        miette!(
            "Failed to update map completion for run {run_id} attempt {attempt} location {:?}: {err}",
            loc
        )
    })?;

    Ok(changed > 0)
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
        let loop_index = db_node
            .loop_index
            .map(|idx| {
                u32::try_from(idx).map_err(|_| {
                    miette!(
                        "Stored loop index {idx} is invalid for run {run_id} attempt {attempt} location {:?}",
                        loc
                    )
                })
            })
            .transpose()?;
        let map_completed = db_node
            .map_completed
            .as_deref()
            .map(decode_map_completed)
            .transpose()?;

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
