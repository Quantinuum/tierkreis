#![allow(missing_docs)]
use crate::location::Location;
use crate::state::schema::{node_outputs, node_states, workflow_runs, workflows};
use chrono::NaiveDateTime;
use chrono::{DateTime, Utc};
use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool, PooledConnection};
use miette::miette;
use std::collections::HashMap;

// -----------------------------------------------------------------------------
// Workflows
// -----------------------------------------------------------------------------

#[derive(Queryable, Selectable, Identifiable, Insertable, Debug, Clone)]
#[diesel(table_name = workflows)]
pub struct WorkflowModel {
    pub id: String, // UUID string
    pub name: Option<String>,
    pub created_at: NaiveDateTime,
}

// -----------------------------------------------------------------------------
// Workflow Runs
// -----------------------------------------------------------------------------

#[derive(Queryable, Selectable, Identifiable, Insertable, Associations, Debug, Clone)]
#[diesel(belongs_to(WorkflowModel, foreign_key = workflow_id))]
#[diesel(primary_key(id, attempt))]
#[diesel(table_name = workflow_runs)]
pub struct WorkflowRunModel {
    pub id: String,
    pub attempt: i32,
    pub workflow_id: String,
    pub run_metadata: String, // JSON string
    pub status: String,
    pub started_at: NaiveDateTime,
}

// -----------------------------------------------------------------------------
// Node States
// -----------------------------------------------------------------------------

#[derive(Queryable, Selectable, Identifiable, Insertable, Associations, Debug, Clone)]
#[diesel(belongs_to(WorkflowRunModel, foreign_key = run_id))]
#[diesel(table_name = node_states)]
pub struct NodeStateModel {
    pub id: String,
    pub run_id: String,
    pub attempt: i32,
    pub node_location: String,
    pub scheduled_time: NaiveDateTime,
    pub queued_time: Option<NaiveDateTime>,
    pub running_time: Option<NaiveDateTime>,
    pub complete_time: Option<NaiveDateTime>,
    pub cancelled_time: Option<NaiveDateTime>,
    pub error_time: Option<NaiveDateTime>,
    pub error: Option<String>,
    pub error_detail: Option<String>,
}

// -----------------------------------------------------------------------------
// Node Outputs
// -----------------------------------------------------------------------------

#[derive(Queryable, Selectable, Identifiable, Insertable, Associations, Debug, Clone)]
#[diesel(belongs_to(NodeStateModel, foreign_key = node_state_id))]
#[diesel(table_name = node_outputs)]
pub struct NodeOutputModel {
    pub id: String,
    pub node_state_id: String,
    pub name: String,
    pub asset_location: String,
}

/// [`RunAttemptState`] is the full state of a run.
#[derive(Debug, Default)]
pub struct RunAttemptState {
    pub nodes: HashMap<Location, crate::state::interface::NodeState>,
    pub metadata: HashMap<String, String>,
}

fn parse_location(raw: &str) -> miette::Result<Location> {
    raw.parse()
        .map_err(|err| miette!("Failed to parse node location '{raw}': {err}"))
}

fn serialize_location(loc: &Location) -> String {
    loc.to_string()
}

fn utc_timestamp(ts: NaiveDateTime) -> DateTime<Utc> {
    DateTime::<Utc>::from_naive_utc_and_offset(ts, Utc)
}

pub fn run_attempt_state_or_default(
    run_id: uuid::Uuid,
    attempt: u32,
    connection: &Pool<ConnectionManager<SqliteConnection>>,
) -> miette::Result<RunAttemptState> {
    use crate::state::schema::{
        node_states::dsl as ns, workflow_runs::dsl as wr, workflows::dsl as wf,
    };

    let mut conn = connection
        .get()
        .map_err(|err| miette!("Failed to get SQLite connection from pool: {err}"))?;

    let attempt_i32 = i32::try_from(attempt)
        .map_err(|_| miette!("Attempt value {attempt} does not fit into i32"))?;
    let run_id_str = run_id.to_string();

    let run = wr::workflow_runs
        .filter(wr::id.eq(run_id_str.clone()))
        .filter(wr::attempt.eq(attempt_i32))
        .order(wr::started_at.asc())
        .select(WorkflowRunModel::as_select())
        .first::<WorkflowRunModel>(&mut conn)
        .optional()
        .map_err(|err| miette!("Failed to query workflow run: {err}"))?;

    let run = match run {
        Some(run) => run,
        None => insert_default_run(&run_id_str, attempt_i32, &mut conn)
            .map_err(|err| miette!("Failed to insert default run for metadata update: {err}"))?,
    };

    let metadata =
        serde_json::from_str::<HashMap<String, String>>(&run.run_metadata).map_err(|err| {
            miette!(
                "Failed to parse run metadata JSON for run {}: {err}",
                run.id
            )
        })?;

    let db_nodes = ns::node_states
        .filter(ns::run_id.eq(run.id.clone()))
        .filter(ns::attempt.eq(run.attempt))
        .select(NodeStateModel::as_select())
        .load::<NodeStateModel>(&mut conn)
        .map_err(|err| {
            miette!(
                "Failed to load node states for run {} attempt {}: {err}",
                run.id,
                run.attempt
            )
        })?;

    let mut nodes = HashMap::new();
    for db_node in db_nodes {
        let node = crate::state::interface::NodeState {
            scheduled_time: Some(utc_timestamp(db_node.scheduled_time)),
            queued_time: db_node.queued_time.map(utc_timestamp),
            running_time: db_node.running_time.map(utc_timestamp),
            complete_time: db_node.complete_time.map(utc_timestamp),
            cancelled_time: db_node.cancelled_time.map(utc_timestamp),
            error_time: db_node.error_time.map(utc_timestamp),
            error: db_node.error.clone(),
            error_detail: db_node.error_detail.clone(),
            ..Default::default()
        };

        let loc = parse_location(&db_node.node_location)?;
        nodes.insert(loc, node);
    }

    Ok(RunAttemptState { nodes, metadata })
}

pub fn update_node_state(
    state: &mut crate::state::interface::NodeState,
    loc: &Location,
    run_id: uuid::Uuid,
    attempt: u32,
    connection: &Pool<ConnectionManager<SqliteConnection>>,
) -> miette::Result<()> {
    use crate::state::schema::node_states::dsl as ns;
    use diesel::upsert::excluded;

    let mut conn = connection
        .get()
        .map_err(|err| miette!("Failed to get SQLite connection from pool: {err}"))?;

    let attempt_i32 = i32::try_from(attempt)
        .map_err(|_| miette!("Attempt value {attempt} does not fit into i32"))?;
    let loc_str = serialize_location(&loc);

    diesel::insert_into(ns::node_states)
        .values((
            ns::id.eq(uuid::Uuid::now_v7().to_string()),
            ns::run_id.eq(run_id.to_string()),
            ns::attempt.eq(attempt_i32),
            ns::node_location.eq(loc_str.clone()),
            ns::scheduled_time.eq(state.scheduled_time.unwrap_or_else(Utc::now).naive_utc()),
            ns::queued_time.eq(state.queued_time.map(|t| t.naive_utc())),
            ns::running_time.eq(state.running_time.map(|t| t.naive_utc())),
            ns::complete_time.eq(state.complete_time.map(|t| t.naive_utc())),
            ns::cancelled_time.eq(state.cancelled_time.map(|t| t.naive_utc())),
            ns::error_time.eq(state.error_time.map(|t| t.naive_utc())),
            ns::error.eq(state.error.clone()),
            ns::error_detail.eq(state.error_detail.clone()),
        ))
        .on_conflict((ns::run_id, ns::attempt, ns::node_location))
        .do_update()
        .set((
            ns::scheduled_time.eq(excluded(ns::scheduled_time)),
            ns::queued_time.eq(excluded(ns::queued_time)),
            ns::running_time.eq(excluded(ns::running_time)),
            ns::complete_time.eq(excluded(ns::complete_time)),
            ns::cancelled_time.eq(excluded(ns::cancelled_time)),
            ns::error_time.eq(excluded(ns::error_time)),
            ns::error.eq(excluded(ns::error)),
            ns::error_detail.eq(excluded(ns::error_detail)),
        ))
        .execute(&mut conn)
        .map_err(|err| miette!("Failed to upsert node state for run {run_id} attempt {attempt} location {:?}: {err}", loc))?;

    Ok(())
}

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
    let loc_str = serialize_location(loc);

    let db_node = ns::node_states
        .filter(ns::run_id.eq(run_id.to_string()))
        .filter(ns::attempt.eq(attempt_i32))
        .filter(ns::node_location.eq(loc_str.clone()))
        .first::<NodeStateModel>(&mut conn)
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
            scheduled_time: Some(utc_timestamp(db_node.scheduled_time)),
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

pub fn add_run_metadata(
    run_id: uuid::Uuid,
    attempt: u32,
    new_metadata: HashMap<String, String>,
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
        .first::<WorkflowRunModel>(&mut conn);
    let run = match run {
        Ok(run) => run,
        Err(diesel::result::Error::NotFound) => {
            insert_default_run(&run_id_str, attempt_i32, &mut conn)
                .map_err(|err| miette!("Failed to insert default run for metadata update: {err}"))?
        }
        Err(err) => {
            return Err(miette!(
                "Failed to query workflow run for metadata update: {err}"
            ));
        }
    };

    let mut metadata =
        serde_json::from_str::<HashMap<String, String>>(&run.run_metadata).map_err(|err| {
            miette!(
                "Failed to parse existing run metadata JSON for run {}: {err}",
                run.id
            )
        })?;

    metadata.extend(new_metadata);

    let updated_metadata_json = serde_json::to_string(&metadata).map_err(|err| {
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
        .first::<WorkflowRunModel>(&mut conn)
        .map_err(|err| miette!("Failed to query workflow run for metadata update: {err}"))?;

    let metadata =
        serde_json::from_str::<HashMap<String, String>>(&run.run_metadata).map_err(|err| {
            miette!(
                "Failed to parse existing run metadata JSON for run {}: {err}",
                run.id
            )
        })?;

    Ok(metadata)
}

fn insert_default_run(
    run_id_str: &String,
    attempt: i32,
    connection: &mut PooledConnection<ConnectionManager<SqliteConnection>>,
) -> miette::Result<WorkflowRunModel> {
    use crate::state::schema::{workflow_runs::dsl as wr, workflows::dsl as wf};
    let now = Utc::now().naive_utc();
    let workflow_id = uuid::Uuid::nil().to_string(); // TODO: Get actual workflow ID if possible.
    diesel::insert_or_ignore_into(wf::workflows)
        .values((
            wf::id.eq(workflow_id.clone()),
            wf::name.eq::<Option<String>>(None),
            wf::created_at.eq(now),
        ))
        .execute(connection)
        .map_err(|err| {
            miette!("Failed to insert default workflow row for run {run_id_str}: {err}")
        })?;

    diesel::insert_into(wr::workflow_runs)
        .values((
            wr::id.eq(run_id_str.clone()),
            wr::attempt.eq(attempt),
            wr::workflow_id.eq(workflow_id.clone()),
            wr::run_metadata.eq("{}".to_string()),
            wr::status.eq("created".to_string()), // What to use for scheduled?
            wr::started_at.eq(now),
        ))
        .on_conflict((wr::id, wr::attempt))
        .do_nothing()
        .execute(connection)
        .map_err(|err| {
            miette!(
                "Failed to insert default workflow run row for run {run_id_str} attempt {attempt}: {err}"
            )
        })?;
    Ok(WorkflowRunModel {
        id: run_id_str.to_string(),
        attempt: attempt,
        workflow_id: workflow_id,
        run_metadata: "{}".to_string(),
        status: "created".to_string(),
        started_at: now,
    })
}
