#![allow(missing_docs)]
use crate::location::Location;
use crate::state::schema::{node_outputs, node_states, workflow_runs, workflows};
use chrono::NaiveDateTime;
use chrono::{DateTime, Utc};
use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool, PooledConnection};
use miette::miette;
use std::collections::HashMap;
use std::hash::BuildHasher;

// -----------------------------------------------------------------------------
// Workflows
// -----------------------------------------------------------------------------

#[derive(Queryable, Selectable, Identifiable, Insertable, Debug, Clone, AsChangeset)]
#[diesel(table_name = workflows)]
pub struct Workflow {
    pub id: String, // UUID string
    pub name: Option<String>,
    pub created_time: Option<NaiveDateTime>,
}

// -----------------------------------------------------------------------------
// Workflow Runs
// -----------------------------------------------------------------------------

#[derive(
    Queryable, Selectable, Identifiable, Insertable, Associations, Debug, Clone, AsChangeset,
)]
#[diesel(belongs_to(Workflow, foreign_key = workflow_id))]
#[diesel(primary_key(id, attempt))]
#[diesel(table_name = workflow_runs)]
pub struct WorkflowRunModel {
    pub id: String, // UUID string
    pub attempt: i32,
    pub workflow_id: String,
    pub run_metadata: Vec<u8>, // JSON blob
    pub status: Option<String>,
    pub started_time: Option<NaiveDateTime>,
}

// -----------------------------------------------------------------------------
// Node States
// -----------------------------------------------------------------------------

#[derive(
    Queryable, Selectable, Identifiable, Insertable, Associations, Debug, Clone, AsChangeset,
)]
#[diesel(belongs_to(WorkflowRunModel, foreign_key = run_id))]
#[diesel(table_name = node_states)]
pub struct NodeStateModel {
    pub id: i32,
    pub run_id: String,
    pub attempt: i32,
    pub node_location: String,
    pub scheduled_time: Option<NaiveDateTime>,
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

#[derive(
    Queryable, Selectable, Identifiable, Insertable, Associations, Debug, Clone, AsChangeset,
)]
#[diesel(belongs_to(NodeStateModel, foreign_key = node_state_id))]
#[diesel(table_name = node_outputs)]
pub struct NodeOutputModel {
    pub id: i32,
    pub node_state_id: i32,
    pub name: String,
    pub asset_kind: String,
    pub storage_name: String,
    pub asset_key: String,
}


