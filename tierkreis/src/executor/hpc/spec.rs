//! HPC Scheduler related functionality.

use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::Arc,
};

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
    /// Explicit job output path handed to the scheduler.
    pub output_path: Option<PathBuf>,
    /// Explicit job error path handed to the.
    pub error_path: Option<PathBuf>,
    /// Additional native scheduler options.
    pub extra_scheduler_args: HashMap<String, Option<String>>,
}

#[derive(Clone, Debug, PartialEq, Default, Deserialize, Serialize)]
/// [`HPCResourceSpec`] determines what Resources should be available to the
/// [`HPCExecutor`] or what is requested as part of a [`TaskPlan`].
pub struct HPCResourceSpec {
    /// Number of compute nodes requested.
    pub nodes: u32,
    /// Number of CPU cores requested on each node.
    pub cores_per_node: Option<u32>,
    /// Memory requested on each node, in gibibytes.
    pub memory_per_node_gb: Option<u32>,
    /// Number of GPUs requested on each node.
    pub gpus_per_node: Option<u32>,
    /// Identifiers of QPUs requested for the job.
    pub qpus: Option<Vec<String>>,
    /// Scheduler-specific generic resources requested for the job.
    pub gres: Option<Vec<String>>,
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
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct UserSpec {
    /// Email address for scheduler notifications.
    pub mail: Option<String>,
}

/// MPI resource settings.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct MpiSpec {
    /// Total MPI processes.
    pub proc: Option<String>,
    /// MPI processes per node.
    pub max_proc_per_node: Option<String>,
    /// Command prefix used to launch MPI programs.
    ///
    /// Defaults to `["mpiexec"]` when unset. The batch script prepends this
    /// launcher to the worker command within the scheduler allocation.
    /// An empty list runs the worker command without an MPI launcher.
    pub launcher: Option<Vec<String>>,
}

/// Default MPI launcher used when [`MpiSpec::launcher`] is unset.
pub const DEFAULT_MPI_LAUNCHER: &str = "mpiexec";

impl MpiSpec {
    /// Launcher arguments, falling back to [`DEFAULT_MPI_LAUNCHER`].
    #[must_use]
    pub fn launcher_args(&self) -> Vec<String> {
        self.launcher
            .clone()
            .unwrap_or_else(|| vec![DEFAULT_MPI_LAUNCHER.to_string()])
    }

    /// Shell-escaped prefix for the worker command in the batch script.
    #[must_use]
    pub fn launch_prefix(&self) -> String {
        self.launcher_args()
            .iter()
            .map(|argument| shell_word(argument))
            .collect::<Vec<_>>()
            .join(" ")
    }
}

/// Quotes a shell word unless it only contains characters that are safe unquoted.
pub(crate) fn shell_word(argument: &str) -> String {
    let is_safe = !argument.is_empty()
        && argument
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || "-_./=:,+@%".contains(c));
    if is_safe {
        argument.to_string()
    } else {
        format!("'{}'", argument.replace('\'', "'\"'\"'"))
    }
}

/// Container settings.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
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
            .add_template("pbs", include_str!("pbs.j2"))
            .expect("embedded PBS template must be valid");
        environment
            .add_template("pjsub", include_str!("pjsub.j2"))
            .expect("embedded PJSUB template must be valid");
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
            .render(minijinja::context! {
                job => minijinja::value::Serde(spec),
                mpi_launch => spec.mpi.as_ref().map(MpiSpec::launch_prefix),
            })
            .into_diagnostic()
    }
}

/// Scheduler-independent state of a submitted job.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SchedulerStatus {
    /// The job is waiting for resources.
    Queued,
    /// The job is executing.
    Running,
    /// The job completed successfully.
    Complete,
    /// The job was cancelled.
    Cancelled,
    /// The job failed.
    Error {
        /// Scheduler-provided failure details.
        message: String,
    },
}

/// Scheduler operations required by the event-based executor.
pub trait SchedulerWrapper: Send + Sync {
    /// Submit a job and return its scheduler job ID.
    fn submit(
        &self,
        spec: JobSpec,
        script_path: &Path,
    ) -> impl Future<Output = Result<String>> + Send;
    /// Check the current states of submitted jobs.
    ///
    /// Jobs that cannot be found are omitted from the returned map.
    fn check(
        &self,
        job_ids: &[&str],
    ) -> impl Future<Output = Result<HashMap<String, SchedulerStatus>>> + Send;
    /// Request cancellation of a job.
    fn cancel(&self, job_id: String) -> impl Future<Output = Result<()>> + Send;
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_job() -> JobSpec {
        JobSpec {
            name: "tierkreis-test".to_string(),
            walltime: "00:15:00".to_string(),
            resources: HPCResourceSpec::new(2, Some(4), Some(8), Some(1), None, None),
            command: "worker args.json".to_string(),
            mpi: Some(MpiSpec {
                proc: Some("8".to_string()),
                max_proc_per_node: Some("4".to_string()),
                launcher: None,
            }),
            queue: Some("debug".to_string()),
            account: Some("project".to_string()),
            ..Default::default()
        }
    }

    #[rstest::rstest]
    #[case::default(None, "mpiexec worker args.json")]
    #[case::slurm(Some(vec!["srun", "--mpi=pmix"]), "srun --mpi=pmix worker args.json")]
    #[case::pbs(Some(vec!["mpirun"]), "mpirun worker args.json")]
    #[case::quoted(Some(vec!["mpiexec", "-x", "A B", "a'b"]), "mpiexec -x 'A B' 'a'\"'\"'b' worker args.json")]
    #[case::disabled(Some(vec![]), "worker args.json")]
    fn renders_launcher_inside_batch_script(
        #[values("pbs", "pjsub", "slurm")] template: &str,
        #[case] launcher: Option<Vec<&str>>,
        #[case] expected: &str,
    ) {
        let mut job = test_job();
        job.mpi.as_mut().expect("job has MPI").launcher =
            launcher.map(|words| words.into_iter().map(String::from).collect());
        let script = ScriptTemplates::default()
            .render(template, &job)
            .expect("template renders");

        assert!(script.starts_with("#!"), "{script}");
        assert_eq!(script.lines().last(), Some(expected), "{script}");
        assert!(!script.contains("TKR_MPI_"), "{script}");
        match template {
            "slurm" => assert!(script.contains("#SBATCH --ntasks=8"), "{script}"),
            "pbs" => assert!(script.contains("#PBS -l mpiprocs=4"), "{script}"),
            "pjsub" => assert!(script.contains("#PJM --mpi \"proc=8\""), "{script}"),
            _ => unreachable!(),
        }
    }

    #[rstest::rstest]
    fn renders_non_mpi_worker(#[values("pbs", "pjsub", "slurm")] template: &str) {
        let mut job = test_job();
        job.mpi = None;
        let script = ScriptTemplates::default()
            .render(template, &job)
            .expect("template renders");

        assert_eq!(script.lines().last(), Some("worker args.json"), "{script}");
        assert!(!script.contains("mpiexec"), "{script}");
    }

    #[test]
    fn renders_pbs_template() {
        let script = ScriptTemplates::default()
            .render("pbs", &test_job())
            .expect("PBS template renders");

        assert!(script.contains("#PBS -N tierkreis-test"));
        assert!(script.contains("#PBS -l walltime=00:15:00"));
        assert!(script.contains("#PBS -l nodes=2"));
        assert!(script.contains("#PBS -l ncpus=4"));
        assert!(script.contains("#PBS -l mem=8gb"));
        assert!(script.contains("#PBS -l ngpus=1"));
        assert!(script.contains("#PBS -l mpiprocs=4"));
        assert!(script.contains("mpiexec worker args.json"));
    }

    #[test]
    fn renders_pjsub_template() {
        let script = ScriptTemplates::default()
            .render("pjsub", &test_job())
            .expect("PJSUB template renders");

        assert!(script.contains("#PJM -N tierkreis-test"));
        assert!(script.contains("#PJM -L \"elapse=00:15:00\""));
        assert!(script.contains("#PJM -L \"node=2\""));
        assert!(script.contains("#PJM --mpi \"max-proc-per-node=4\""));
        assert!(script.contains("mpiexec worker args.json"));
    }
}
