//! HPC Scheduler related functionality.

use std::{collections::HashMap, path::PathBuf, sync::Arc};

use futures::future::BoxFuture;
use miette::{IntoDiagnostic, Result};
use serde::{Deserialize, Serialize};

/// Scheduler-independent job description.
/// TODOs: make sure non optional values are not ""
#[derive(Clone, Debug, Default, Serialize)]
pub struct JobSpec {
    /// Scheduler job name.
    pub name: String,
    /// Maximum wall-clock time, in `HH:MM:SS` format.
    pub walltime: String,
    /// Job resources specification.
    pub resources: HPCResourceSpec,
    /// Scheduler partition or queue.
    pub queue: Option<String>,
    /// Scheduler account or project.
    pub account: Option<String>,
    /// User-specific scheduler settings.
    pub user: Option<UserSpec>,
    /// MPI settings.
    pub mpi: Option<MpiSpec>,
    /// Container settings.
    pub container: Option<ContainerSpec>,
    /// Opaque command executed by the scheduler.
    pub command: String,
    /// Environment exported by the job script.
    pub environment: HashMap<String, String>,
    /// Modules loaded before the command.
    pub modules: Vec<String>,
    /// Explicit scheduler output path.
    pub output_path: Option<PathBuf>,
    /// Explicit scheduler error path.
    pub error_path: Option<PathBuf>,
    /// Additional native scheduler options.
    pub extra_scheduler_args: HashMap<String, Option<String>>,
}

#[derive(Clone, Debug, PartialEq, Default, Deserialize, Serialize)]
/// [`HPCResourceSpec`] determines what Resources should be available to the
/// [`HPCExecutor`] or what is requested as part of a [`TaskPlan`].
pub struct HPCResourceSpec {
    nodes: u32,
    cores_per_node: Option<u32>,
    memory_per_node_gb: Option<u32>,
    gpus_per_node: Option<u32>,
    qpus: Option<Vec<String>>,
    gres: Option<Vec<String>>,
}

impl HPCResourceSpec {
    /// Creates a new [`HPCResourceSpec`] with the given resource specifications.
    #[must_use]
    pub fn new(
        nodes: u32,
        cores_per_node: Option<u32>,
        memory_per_node_gb: Option<u32>,
        gpus_per_node: Option<u32>,
        qpus: Option<Vec<String>>,
        gres: Option<Vec<String>>,
    ) -> Self {
        Self {
            nodes,
            cores_per_node,
            memory_per_node_gb,
            gpus_per_node,
            qpus,
            gres,
        }
    }
    /// Checks if the current [`HPCResourceSpec`] satisfies the requirements of another [`HPCResourceSpec`].
    /// TODO: gres and qpus
    #[must_use]
    pub fn satisfies(&self, other: &HPCResourceSpec) -> bool {
        self.nodes >= other.nodes
            && self.cores_per_node.unwrap_or(0) >= other.cores_per_node.unwrap_or(0)
            && self.memory_per_node_gb.unwrap_or(0) >= other.memory_per_node_gb.unwrap_or(0)
            && self.gpus_per_node.unwrap_or(0) >= other.gpus_per_node.unwrap_or(0)
    }
}

/// User-specific scheduler settings.
#[derive(Clone, Debug, Default, Serialize)]
pub struct UserSpec {
    /// Email address for scheduler notifications.
    pub mail: Option<String>,
}

/// MPI resource settings.
#[derive(Clone, Debug, Default, Serialize)]
pub struct MpiSpec {
    /// Total MPI processes.
    pub proc: Option<String>,
    /// MPI processes per node.
    pub max_proc_per_node: Option<String>,
}

/// Container settings.
#[derive(Clone, Debug, Default, Serialize)]
pub struct ContainerSpec {
    /// Container image.
    pub image: String,
    /// Container engine.
    pub engine: String,
    /// Optional container name.
    pub name: Option<String>,
    /// Engine-specific arguments.
    pub extra_args: HashMap<String, Option<String>>,
    /// Optional environment file.
    pub env_file: Option<PathBuf>,
}

/// Named submission templates
#[derive(Clone, Debug)]
pub struct ScriptTemplates {
    environment: Arc<minijinja::Environment<'static>>,
}

impl Default for ScriptTemplates {
    fn default() -> Self {
        let mut environment = minijinja::Environment::new();
        environment.set_auto_escape_callback(|_| minijinja::AutoEscape::None);
        environment
            .add_template("slurm", include_str!("slurm.j2"))
            .expect("embedded Slurm template must be valid");
        Self {
            environment: Arc::new(environment),
        }
    }
}

impl ScriptTemplates {
    /// Render a job submission template.
    ///
    /// # Errors
    ///
    /// If the template cannot be found or rendering fails.
    pub fn render(&self, name: &str, spec: &JobSpec) -> Result<String> {
        self.environment
            .get_template(name)
            .into_diagnostic()?
            .render(minijinja::context! { job => spec })
            .into_diagnostic()
    }
}

/// Scheduler operations required by the event-based executor.
pub trait SchedulerWrapper: Send + Sync {
    /// Submit a job and return its scheduler job ID.
    fn submit(&self, spec: JobSpec) -> BoxFuture<'_, Result<String>>;
    /// Wait for a submitted job to finish.
    fn wait(&self, job_id: String) -> BoxFuture<'_, Result<()>>;
    /// Request cancellation of a job.
    fn cancel(&self, job_id: String) -> BoxFuture<'_, Result<()>>;
}
