use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::executor::nexus::client::models::NewRelationship;

#[derive(Debug, Serialize)]
struct NewJobRelationships {
    project: NewRelationship,
}

impl NewJobRelationships {
    pub fn new(project_id: Uuid) -> Self {
        Self {
            project: NewRelationship::new(project_id, "project"),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct JobProperties {}

#[derive(Debug, Serialize)]
#[serde(tag = "type")]
pub enum NewBackendConfig {
    SeleneConfig {},
}

impl NewBackendConfig {
    fn selene() -> NewBackendConfig {
        NewBackendConfig::SeleneConfig {}
    }
}

#[derive(Debug, Serialize)]
pub struct NewExecuteJobItem {
    program_id: Uuid,
    n_shots: u64,
    n_qubits: u64,
}

impl NewExecuteJobItem {
    pub fn new(program_id: Uuid, n_shots: u64) -> Self {
        NewExecuteJobItem {
            program_id,
            n_shots,
            n_qubits: 20,
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(tag = "job_definition_type")]
#[serde(rename_all = "snake_case")]
pub enum NewJobDefinition<'a> {
    ExecuteJobDefinition {
        backend_config: NewBackendConfig,
        items: &'a [NewExecuteJobItem],
    },
}

impl NewJobDefinition<'_> {
    pub fn new_execute(items: &[NewExecuteJobItem]) -> NewJobDefinition<'_> {
        NewJobDefinition::ExecuteJobDefinition {
            backend_config: NewBackendConfig::selene(),
            items,
        }
    }

    fn job_type(&self) -> &'static str {
        match self {
            Self::ExecuteJobDefinition { .. } => "execute",
        }
    }
}

#[derive(Debug, Serialize)]
struct NewJobAttributes<'a> {
    name: &'a str,
    description: Option<&'a str>,
    properties: JobProperties,
    job_type: &'static str,
    definition: NewJobDefinition<'a>,
}

#[derive(Debug, Serialize)]
struct NewJobData<'a> {
    attributes: NewJobAttributes<'a>,
    relationships: NewJobRelationships,
    r#type: &'static str,
}

#[derive(Debug, Serialize)]
pub struct NewJob<'a> {
    data: NewJobData<'a>,
}

impl NewJob<'_> {
    pub fn new<'a>(
        name: &'a str,
        description: Option<&'a str>,
        project_id: Uuid,
        definition: NewJobDefinition<'a>,
    ) -> NewJob<'a> {
        NewJob {
            data: NewJobData {
                attributes: NewJobAttributes {
                    name,
                    description,
                    properties: JobProperties {},
                    job_type: definition.job_type(),
                    definition,
                },
                relationships: NewJobRelationships::new(project_id),
                r#type: "job",
            },
        }
    }
}

#[derive(Debug, Serialize)]
pub struct CancelOptions {}

#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum StatusEnum {
    Completed,
    Queued,
    Submitted,
    Running,
    Cancelled,
    Error,
    Cancelling,
    Retrying,
    Terminated,
    Depleted,
}

#[derive(Debug, Deserialize)]
pub struct Status {
    status: StatusEnum,
    #[allow(unused)]
    message: String,
}

impl Status {
    pub fn status(&self) -> StatusEnum {
        self.status
    }
}

#[derive(Debug, Deserialize)]
pub struct ExecuteJobItem {
    result_id: Option<Uuid>,
}

impl ExecuteJobItem {
    pub fn result_id(&self) -> Option<Uuid> {
        self.result_id
    }
}

#[derive(Debug, Deserialize)]
#[serde(tag = "job_definition_type")]
#[serde(rename_all = "snake_case")]
pub enum JobDefinition {
    ExecuteJobDefinition { items: Vec<ExecuteJobItem> },
}

#[derive(Debug, Deserialize)]
pub struct JobAttributes {
    status: Status,
    definition: JobDefinition,
}

#[derive(Debug, Deserialize)]
pub struct JobData {
    attributes: JobAttributes,
}

impl JobData {
    pub fn status_enum(&self) -> StatusEnum {
        self.attributes.status.status
    }

    pub fn status_message(&self) -> &String {
        &self.attributes.status.message
    }

    pub fn definition(self) -> JobDefinition {
        self.attributes.definition
    }
}

#[derive(Debug, Deserialize)]
pub struct Job {
    data: JobData,
}

impl Job {
    pub fn data(self) -> JobData {
        self.data
    }
}
