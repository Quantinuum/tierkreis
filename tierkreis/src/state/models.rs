#![allow(missing_docs)]
use crate::location::Location;
use crate::state::schema::{
    node_outputs, node_states, workflow_run_attempts, workflow_run_inputs, workflow_runs, workflows, executor_debug,
};
use chrono::NaiveDateTime;
use diesel::prelude::*;

// -----------------------------------------------------------------------------
// Workflows
// -----------------------------------------------------------------------------

#[derive(Insertable, Debug, Clone)]
#[diesel(table_name = workflows)]
pub struct NewWorkflow<'a> {
    pub id: &'a str, // UUID string
    pub name: Option<&'a str>,
    pub definition: &'a [u8], // JSON blob
    pub created_time: Option<NaiveDateTime>,
}

#[derive(Queryable, Selectable, Identifiable, Debug, Clone)]
#[diesel(table_name = workflows)]
pub struct Workflow {
    pub id: String, // UUID string
    pub name: Option<String>,
    pub definition: Vec<u8>, // JSON blob
    pub created_time: Option<NaiveDateTime>,
}

// -----------------------------------------------------------------------------
// Workflow Runs
// -----------------------------------------------------------------------------

#[derive(Insertable, Debug, Clone)]
#[diesel(table_name = workflow_runs)]
pub struct NewWorkflowRun<'a> {
    pub id: &'a str, // UUID string
    pub workflow_id: &'a str,
}

#[derive(Queryable, Selectable, Identifiable, Associations, Debug, Clone)]
#[diesel(belongs_to(Workflow, foreign_key = workflow_id))]
#[diesel(table_name = workflow_runs)]
pub struct WorkflowRun {
    pub id: String, // UUID string
    pub workflow_id: String,
}

// -----------------------------------------------------------------------------
// Workflow Runs Attempts
// -----------------------------------------------------------------------------

#[derive(Insertable, Debug, Clone)]
#[diesel(table_name = workflow_run_attempts)]
pub struct NewWorkflowRunAttempt<'a> {
    pub id: &'a str, // UUID string
    pub attempt: i32,
    pub run_metadata: &'a [u8], // JSON blob
    pub status: Option<&'a str>,
    pub started_time: Option<NaiveDateTime>,
}

#[derive(Queryable, Selectable, Identifiable, Associations, Debug, Clone)]
#[diesel(belongs_to(WorkflowRun, foreign_key = workflow_run_id))]
#[diesel(table_name = workflow_run_attempts)]
pub struct WorkflowRunAttempt {
    pub id: i32,
    pub workflow_run_id: String, // UUID string
    pub attempt: i32,
    pub run_metadata: Vec<u8>, // JSON blob
    pub status: Option<String>,
    pub started_time: Option<NaiveDateTime>,
}

// -----------------------------------------------------------------------------
// Node States
// -----------------------------------------------------------------------------

#[derive(Queryable, Selectable, Identifiable, Insertable, Associations, Debug, Clone)]
#[diesel(belongs_to(WorkflowRun, foreign_key = run_id))]
#[diesel(table_name = node_states)]
pub struct NodeState {
    pub id: i32,
    pub run_id: String,
    pub attempt: i32,
    pub node_location: Location,
    pub scheduled_time: Option<NaiveDateTime>,
    pub queued_time: Option<NaiveDateTime>,
    pub running_time: Option<NaiveDateTime>,
    pub complete_time: Option<NaiveDateTime>,
    pub cancelled_time: Option<NaiveDateTime>,
    pub error_time: Option<NaiveDateTime>,
    pub cond: Option<bool>,
    pub loop_index: Option<i32>,
    pub map_size: Option<i32>,
    pub map_completed: Option<Vec<u8>>,
    pub error: Option<String>,
    pub error_detail: Option<String>,
}

#[derive(Insertable, AsChangeset, Default, Debug)]
#[diesel(belongs_to(WorkflowRun, foreign_key = run_id))]
#[diesel(table_name = crate::state::schema::node_states)]
#[diesel(treat_none_as_default_value = false)]
pub struct UpsertNodeState {
    pub run_id: String,
    pub attempt: i32,
    pub node_location: Location,
    pub scheduled_time: Option<NaiveDateTime>,
    pub queued_time: Option<NaiveDateTime>,
    pub running_time: Option<NaiveDateTime>,
    pub complete_time: Option<NaiveDateTime>,
    pub cancelled_time: Option<NaiveDateTime>,
    pub error_time: Option<NaiveDateTime>,
    pub cond: Option<bool>,
    pub loop_index: Option<i32>,
    pub map_size: Option<i32>,
    pub map_completed: Option<Vec<u8>>,
    pub error: Option<String>,
    pub error_detail: Option<String>,
}

// -----------------------------------------------------------------------------
// Workflow Run Inputs
// -----------------------------------------------------------------------------

#[derive(Queryable, Selectable, Identifiable, Associations, Debug, Clone)]
#[diesel(belongs_to(WorkflowRun, foreign_key = workflow_run_id))]
#[diesel(table_name = workflow_run_inputs)]
pub struct WorkflowRunInput {
    pub id: i32,
    pub workflow_run_id: String,
    pub name: String,
    pub asset_kind: String,
    pub storage_name: String,
    pub asset_key: String,
}

#[derive(Insertable, Debug, Clone)]
#[diesel(table_name = workflow_run_inputs)]
pub struct NewWorkflowRunInput<'a> {
    pub workflow_run_id: &'a str,
    pub name: &'a str,
    pub asset_kind: String,
    pub storage_name: &'a str,
    pub asset_key: String,
}

// -----------------------------------------------------------------------------
// Node Outputs
// -----------------------------------------------------------------------------

#[derive(Queryable, Selectable, Identifiable, Insertable, Associations, Debug, Clone)]
#[diesel(belongs_to(NodeState, foreign_key = node_state_id))]
#[diesel(table_name = node_outputs)]
pub struct NodeOutput {
    pub id: i32,
    pub node_state_id: i32,
    pub name: String,
    pub asset_kind: String,
    pub storage_name: String,
    pub asset_key: String,
}

#[derive(Insertable, Debug, Clone)]
#[diesel(table_name = node_outputs)]
pub struct NewNodeOutput<'a> {
    pub name: &'a str,
    pub asset_kind: String,
    pub storage_name: &'a str,
    pub asset_key: String,
}


// -----------------------------------------------------------------------------
// Executor Debug Data
// -----------------------------------------------------------------------------

#[derive(Queryable, Selectable, Identifiable, Insertable, Debug, Clone)]
#[diesel(table_name = executor_debug)]
#[diesel(belongs_to(NodeState, foreign_key = node_state_id))]
pub struct ExecutorDebugData {
    pub id: i32,
    pub node_state_id: i32,
    pub executor_name: String,
    pub worker_name: String,
    pub task_name: String,
    pub resources: Vec<u8>, // JSON blob
    pub environment: Vec<u8>, // JSON blob
    pub internal_id: Option<String>,
}
