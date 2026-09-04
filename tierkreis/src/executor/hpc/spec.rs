//! HPC Scheduler related functionality.

use std::{
    collections::{BTreeMap, HashMap},
    path::PathBuf,
    sync::Arc,
};

use futures::future::BoxFuture;
use miette::{IntoDiagnostic, Result};
use serde::Serialize;

/// Scheduler-independent job description.
/// TODOs: make sure non optional values are not ""
#[derive(Clone, Debug, Default, Serialize)]
pub struct JobSpec {
    /// Scheduler job name.
    pub name: String,
    /// Maximum wall-clock time, in `HH:MM:SS` format.
    pub walltime: String,
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
    pub extra_scheduler_args: BTreeMap<String, Option<String>>,
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
    pub extra_args: BTreeMap<String, Option<String>>,
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
        environment
            .add_template("pbs", include_str!("pbs.j2"))
            .expect("embedded PBS template must be valid");
        Self {
            environment: Arc::new(environment),
        }
    }
}

impl ScriptTemplates {
    /// Render a job submission template.
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
