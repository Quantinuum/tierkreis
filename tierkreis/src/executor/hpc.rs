//! Event-based execution of subprocess-compatible workers on a batch scheduler.

pub mod pbs;
pub mod pjsub;
pub mod slurm;
pub mod spec;

use std::{
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    time::Duration,
};

use futures::{FutureExt, SinkExt, StreamExt, channel::mpsc, future::BoxFuture, stream::BoxStream};
use miette::{Context, IntoDiagnostic, miette};
use regex::Regex;
use serde::{Deserialize, Serialize};
use tokio::{task::AbortHandle, time::MissedTickBehavior};
use tracing::{instrument, warn};
use tracing_opentelemetry::OpenTelemetrySpanExt;
use uuid::Uuid;
use which::which_re;

use crate::{
    asset_storage::{AssetSpec, AssetStorageRegistry, reserve_asset_specs, transfer_assets},
    event::{
        EventReceiver, EventSender, RuntimeEvent, send_cancelled, send_complete, send_error,
        send_queued, send_running,
    },
    executor::{
        hpc::spec::{
            ContainerSpec, HPCResourceSpec, JobSpec, MpiSpec, SchedulerStatus, SchedulerWrapper,
            UserSpec,
        },
        interface::{Executor, TaskHandle, TaskPlan, WorkerSpec},
    },
    location::Location,
    monitoring::{EnvironmentCarrier, inject_trace_context},
    resource_profiles::downcast,
};

fn shell_quote(argument: &str) -> String {
    format!("'{}'", argument.replace('\'', "'\"'\"'"))
}

/// Builds the worker invocation command escaping each argument for the shell.
/// Falls back to `tkr-<worker_name>` if no command is specified.
/// Otherwise uses the command specified for the worker in the commands map.
/// For example:
///
/// ```toml
/// [executors.slurm.worker_commands]
/// mpi_worker = ["uv", "run", "/mpi_worker/main.py"]
/// ```
///
/// This invokes `["uv", "run", "/mpi_worker/main.py", "<worker_args_path>"]`.
fn worker_invocation(
    commands: &HashMap<String, Vec<String>>,
    worker_name: &str,
    worker_args_path: &Path,
) -> String {
    let default = vec![format!("tkr-{}", worker_name.replace('_', "-"))];
    commands
        .get(worker_name)
        .unwrap_or(&default)
        .iter()
        .map(|argument| shell_quote(argument))
        .chain(std::iter::once(shell_quote(
            &worker_args_path.to_string_lossy(),
        )))
        .collect::<Vec<_>>()
        .join(" ")
}

#[derive(serde::Deserialize, Default)]
#[serde(default)]
struct RawResourceRequest {
    nodes: Option<u32>,
    cpu: Option<f64>,
    memory: Option<String>,
    gpu: Option<u32>,
    walltime: Option<String>,
    queue: Option<String>,
    account: Option<String>,
    modules: Vec<String>,
    gres: Vec<String>,
    qpus: Vec<String>,
    user: Option<UserSpec>,
    mpi: Option<MpiSpec>,
    container: Option<ContainerSpec>,
    extra_scheduler_args: HashMap<String, Option<String>>,
    extras: HashMap<String, serde_json::Value>,
}

struct ResourceRequest {
    resources: HPCResourceSpec,
    walltime: Option<String>,
    queue: Option<String>,
    account: Option<String>,
    modules: Vec<String>,
    user: Option<UserSpec>,
    mpi: Option<MpiSpec>,
    container: Option<ContainerSpec>,
    extra_scheduler_args: HashMap<String, Option<String>>,
}

impl TryFrom<HashMap<String, serde_json::Value>> for ResourceRequest {
    type Error = miette::Report;

    fn try_from(resources: HashMap<String, serde_json::Value>) -> Result<Self, Self::Error> {
        let raw: RawResourceRequest =
            serde_json::from_value(serde_json::Value::Object(resources.into_iter().collect()))
                .into_diagnostic()
                .wrap_err("Invalid HPC resource specification")?;
        let cores_per_node = raw.cpu.map(downcast);
        let memory_per_node_gb = raw
            .memory
            .as_deref()
            .map(|memory| {
                let gib = memory
                    .strip_suffix("Gi")
                    .ok_or_else(|| miette!("HPC memory must use Gi units"))?;
                gib.parse::<u32>()
                    .into_diagnostic()
                    .wrap_err("Invalid HPC memory quantity")
            })
            .transpose()?;
        let mut extra_scheduler_args = raw.extra_scheduler_args;
        if let Some(flags) = raw.extras.get("flags") {
            let flags: Vec<String> = serde_json::from_value(flags.clone())
                .into_diagnostic()
                .wrap_err("HPC extras.flags must be a list of strings")?;
            for flag in flags {
                extra_scheduler_args.insert(flag, None);
            }
        }

        Ok(Self {
            resources: HPCResourceSpec {
                nodes: raw.nodes.unwrap_or_default(),
                cores_per_node,
                memory_per_node_gb,
                gpus_per_node: raw.gpu,
                qpus: (!raw.qpus.is_empty()).then_some(raw.qpus),
                gres: (!raw.gres.is_empty()).then_some(raw.gres),
            },
            walltime: raw.walltime,
            queue: raw.queue,
            account: raw.account,
            modules: raw.modules,
            user: raw.user,
            mpi: raw.mpi,
            container: raw.container,
            extra_scheduler_args,
        })
    }
}

#[derive(Serialize, Deserialize, Default)]
struct WorkerCallArgs {
    function_name: String,
    inputs: HashMap<String, PathBuf>,
    outputs: HashMap<String, PathBuf>,
    output_dir: PathBuf,
    done_path: PathBuf,
    error_path: PathBuf,
    logs_path: Option<PathBuf>,
}

struct BackgroundTaskPlan {
    workflow_run_id: Uuid,
    attempt: u32,
    loc: Location,
    outputs: HashMap<String, AssetSpec>,
    output_storage_name: String,
    job_id: TaskHandle,
    _worker_args: PathBuf,
}

struct BackgroundTask {
    workflow_run_id: Uuid,
    attempt: u32,
    loc: Location,
    outputs: HashMap<String, AssetSpec>,
    output_storage_name: String,
    _worker_args: PathBuf,
}

enum TaskRegistration {
    Watching {
        job_id: String,
        task: BackgroundTask,
        status: Option<SchedulerStatus>,
    },
    CancelBeforeRegistration,
}

type Key = (Uuid, u32, Location);
type TaskReceiver = mpsc::Receiver<BackgroundTaskPlan>;
type CancelReceiver = mpsc::Receiver<Key>;
type JobHandles = HashMap<Key, TaskRegistration>;
type OutputSpecs = (HashMap<String, AssetSpec>, HashMap<String, PathBuf>);

// Identical to nexus
#[instrument(skip_all, err)]
async fn process_cancelled_task<T: SchedulerWrapper>(
    scheduler: Arc<T>,
    job_handles: &mut JobHandles,
    workflow_run_id: Uuid,
    attempt: u32,
    loc: Location,
) -> miette::Result<()> {
    let key = (workflow_run_id, attempt, loc.clone());
    let handle = job_handles.remove(&key);
    match handle {
        // We know the job id, cancel the job.
        Some(TaskRegistration::Watching { job_id, .. }) => {
            tracing::info!("Cancelling job for node: {key:?}");
            scheduler.cancel(job_id.clone()).await?;
        }
        // We are already planning to cancel the job if we see this node.
        Some(TaskRegistration::CancelBeforeRegistration) => {}
        // Register that the node needs to be cancelled if we see it.
        None => {
            warn!("Got cancel request before registering job_id for node: {key:?}");
            job_handles.insert(key, TaskRegistration::CancelBeforeRegistration);
        }
    }
    Ok(())
}

#[instrument(skip_all, err)]
async fn process_finished_task(
    event_sender: &mut EventSender,
    job_handles: &mut JobHandles,
    asset_storage_registry: &AssetStorageRegistry,
    background_task: BackgroundTask,
    status: SchedulerStatus,
) -> miette::Result<()> {
    let loc = background_task.loc;
    let outputs = background_task.outputs;
    let output_storage_name = background_task.output_storage_name;
    let workflow_run_id = background_task.workflow_run_id;
    let attempt = background_task.attempt;

    job_handles.remove(&(workflow_run_id, attempt, loc.clone()));

    match status {
        SchedulerStatus::Complete => {
            let outputs =
                transfer_assets(asset_storage_registry, &output_storage_name, &outputs).await;
            match outputs {
                Ok(outputs) => {
                    send_complete(event_sender, workflow_run_id, attempt, vec![(loc, outputs)])
                        .await?;
                }
                Err(err) => {
                    send_error(event_sender, workflow_run_id, attempt, loc, &err).await?;
                }
            }
        }
        SchedulerStatus::Cancelled => {
            send_cancelled(event_sender, workflow_run_id, attempt, loc).await?;
        }
        SchedulerStatus::Error { message } => {
            send_error(
                event_sender,
                workflow_run_id,
                attempt,
                loc,
                &miette!(message),
            )
            .await?;
        }
        SchedulerStatus::Queued | SchedulerStatus::Running => unreachable!(),
    }

    Ok(())
}

#[instrument(skip_all, err)]
async fn monitor_task<T: SchedulerWrapper>(
    scheduler: Arc<T>,
    event_sender: &EventSender,
    job_handles: &mut JobHandles,
    internal_task: BackgroundTaskPlan,
) -> miette::Result<()> {
    let BackgroundTaskPlan {
        workflow_run_id,
        attempt,
        loc,
        outputs,
        job_id,
        output_storage_name,
        _worker_args: worker_args,
    } = internal_task;
    let mut queued_event_sender = event_sender.clone();
    send_queued(
        &mut queued_event_sender,
        workflow_run_id,
        attempt,
        loc.clone(),
        Some(job_id.clone()),
    )
    .await?;

    let key = (workflow_run_id, attempt, loc.clone());
    let should_cancel = matches!(
        job_handles.get(&key),
        Some(TaskRegistration::CancelBeforeRegistration)
    );
    job_handles.insert(
        key,
        TaskRegistration::Watching {
            job_id: job_id.clone(),
            task: BackgroundTask {
                workflow_run_id,
                attempt,
                loc,
                outputs,
                output_storage_name,
                _worker_args: worker_args,
            },
            status: None,
        },
    );
    if should_cancel {
        scheduler.cancel(job_id).await?;
    }
    Ok(())
}

async fn check_jobs<T: SchedulerWrapper>(
    scheduler: &Arc<T>,
    event_sender: &mut EventSender,
    job_handles: &mut JobHandles,
    asset_storage_registry: &AssetStorageRegistry,
) -> miette::Result<()> {
    let jobs: Vec<_> = job_handles
        .iter()
        .filter_map(|(key, status)| match status {
            TaskRegistration::Watching { job_id, .. } => Some((key.clone(), job_id.clone())),
            TaskRegistration::CancelBeforeRegistration => None,
        })
        .collect();
    if jobs.is_empty() {
        return Ok(());
    }

    let statuses = scheduler
        .check(
            &jobs
                .iter()
                .map(|(_, job_id)| job_id.as_str())
                .collect::<Vec<_>>(),
        )
        .await?;
    for (key, job_id) in jobs {
        let Some(status) = statuses.get(&job_id).cloned() else {
            continue;
        };
        match status {
            SchedulerStatus::Queued | SchedulerStatus::Running => {
                let TaskRegistration::Watching {
                    task,
                    status: previous_status,
                    ..
                } = job_handles.get_mut(&key).expect("job handle must exist")
                else {
                    continue;
                };
                if status == SchedulerStatus::Running
                    && previous_status.as_ref() != Some(&SchedulerStatus::Running)
                {
                    send_running(
                        event_sender,
                        task.workflow_run_id,
                        task.attempt,
                        task.loc.clone(),
                    )
                    .await?;
                }
                *previous_status = Some(status);
            }
            SchedulerStatus::Complete
            | SchedulerStatus::Cancelled
            | SchedulerStatus::Error { .. } => {
                let Some(TaskRegistration::Watching { task, .. }) = job_handles.remove(&key) else {
                    continue;
                };
                process_finished_task(
                    event_sender,
                    job_handles,
                    asset_storage_registry,
                    task,
                    status,
                )
                .await?;
            }
        }
    }
    Ok(())
}

async fn process_tasks<T: SchedulerWrapper>(
    scheduler: Arc<T>,
    mut task_receiver: TaskReceiver,
    mut cancel_receiver: CancelReceiver,
    mut event_sender: EventSender,
    asset_storage_registry: AssetStorageRegistry,
    poll_interval: Duration,
) {
    let mut job_handles: JobHandles = HashMap::new();
    let mut monitor_interval = tokio::time::interval(poll_interval);
    monitor_interval.set_missed_tick_behavior(MissedTickBehavior::Skip);

    loop {
        tokio::select! {
            // A task has been canceled
            Some((workflow_run_id, attempt, loc)) = cancel_receiver.next() => {
                process_cancelled_task(scheduler.clone(), &mut job_handles, workflow_run_id, attempt, loc)
                    .await
                    .expect("Failed to cancel task");
            }
            _ = monitor_interval.tick() => {
                if let Err(error) = check_jobs(
                    &scheduler,
                    &mut event_sender,
                    &mut job_handles,
                    &asset_storage_registry,
                ).await {
                    tracing::error!("Failed to check HPC jobs: {error:?}");
                }
            }
            // A task has been submitted
            Some(task) = task_receiver.next() => {
                let workflow_run_id = task.workflow_run_id;
                let attempt = task.attempt;
                let loc = task.loc.clone();
                if let Err(error) = monitor_task(
                    scheduler.clone(),
                    &event_sender,
                    &mut job_handles,
                    task,
                ).await {
                    send_error(&mut event_sender, workflow_run_id, attempt, loc, &error)
                        .await
                        .expect("Failed to send error event");
                }
            }
            else => break,
        }
    }
}

/// Event-based executor for subprocess-compatible workers.
pub struct HPCExecutor<T: SchedulerWrapper> {
    scheduler: Arc<T>,
    task_sender: mpsc::Sender<BackgroundTaskPlan>,
    cancel_sender: mpsc::Sender<Key>,
    event_receiver: Mutex<Option<EventReceiver>>,
    background_abort_handle: AbortHandle,
    hpc_storage_name: String,
    output_storage_name: String,
    asset_storage_registry: AssetStorageRegistry,
    /// Available HPC resources for this executor.
    pub max_resources: HPCResourceSpec,
    worker_commands: HashMap<String, Vec<String>>,
    tkr_tmp_dir: PathBuf,
}

impl<T: SchedulerWrapper> Drop for HPCExecutor<T> {
    fn drop(&mut self) {
        self.background_abort_handle.abort();
    }
}

impl<T: SchedulerWrapper + 'static> HPCExecutor<T> {
    /// Create an HPC executor using shared file-backed storage and worker command overrides.
    ///
    /// # Errors
    ///
    /// If the specified storage names are not present in the registry or a worker
    /// command has no executable.
    pub fn try_new(
        asset_storage_registry: &AssetStorageRegistry,
        hpc_storage_name: &str,
        output_storage_name: &str,
        scheduler: Arc<T>,
        max_resources: HPCResourceSpec,
        poll_interval: Duration,
        // Specific commands to invoke e.g.
        // [executors.slurm.worker_commands]
        // mpi_worker = ["uv", "run", "/mpi_worker/main.py"]
        worker_commands: HashMap<String, Vec<String>>,
    ) -> miette::Result<Self> {
        if !asset_storage_registry.contains_key(hpc_storage_name) {
            return Err(miette!("hpc_storage_name not in registry"));
        }
        if !asset_storage_registry.contains_key(output_storage_name) {
            return Err(miette!("output_storage_name not in registry"));
        }
        for (worker, command) in &worker_commands {
            if command.first().is_none_or(std::string::String::is_empty) {
                return Err(miette!("Worker `{worker}` requires a nonempty executable"));
            }
        }
        let tierkreis_dir = crate::config::tierkreis_home_dir().join("tmp");
        std::fs::create_dir_all(&tierkreis_dir)
            .into_diagnostic()
            .wrap_err_with(|| {
                format!(
                    "Failed to create Tierkreis temporary directory at {}",
                    tierkreis_dir.display()
                )
            })?;

        let (task_sender, task_receiver) = mpsc::channel(64);
        let (event_sender, event_receiver) = mpsc::channel(64);
        let (cancel_sender, cancel_receiver) = mpsc::channel(64);
        let background = tokio::spawn(process_tasks(
            scheduler.clone(),
            task_receiver,
            cancel_receiver,
            event_sender,
            Arc::clone(asset_storage_registry),
            poll_interval,
        ));
        Ok(Self {
            scheduler,
            task_sender,
            cancel_sender,
            event_receiver: Mutex::new(Some(event_receiver)),
            background_abort_handle: background.abort_handle(),
            hpc_storage_name: hpc_storage_name.into(),
            output_storage_name: output_storage_name.into(),
            asset_storage_registry: Arc::clone(asset_storage_registry),
            max_resources,
            worker_commands,
            tkr_tmp_dir: tierkreis_dir,
        })
    }

    fn reserve_tmp_paths(&self, count: usize) -> miette::Result<Vec<PathBuf>> {
        (0..count)
            .map(|_| {
                tempfile::Builder::new()
                    .prefix("tkr-")
                    .tempfile_in(&self.tkr_tmp_dir)
                    .into_diagnostic()?
                    .keep()
                    .into_diagnostic()
                    .map(|(_, path)| path)
            })
            .collect()
    }

    async fn build_outputs(&self, outputs: HashSet<String>) -> miette::Result<OutputSpecs> {
        let output_specs = reserve_asset_specs(
            &self.asset_storage_registry,
            &self.hpc_storage_name,
            outputs.len(),
        )
        .await?;
        let outputs: HashMap<_, _> = outputs.into_iter().zip(output_specs).collect();
        let output_paths = outputs
            .iter()
            .map(|(name, spec)| Ok((name.clone(), spec.path()?)))
            .collect::<miette::Result<HashMap<_, _>>>()?;
        Ok((outputs, output_paths))
    }

    async fn build_worker_call_args(
        &self,
        task: &TaskPlan,
        output_paths: HashMap<String, PathBuf>,
    ) -> miette::Result<WorkerCallArgs> {
        let inputs = transfer_assets(
            &self.asset_storage_registry,
            &self.hpc_storage_name,
            &task.inputs,
        )
        .await?;
        let input_paths = inputs
            .iter()
            .map(|(name, spec)| Ok((name.clone(), spec.path()?)))
            .collect::<miette::Result<HashMap<_, _>>>()?;
        Ok(WorkerCallArgs {
            function_name: task.task_name.clone(),
            inputs: input_paths,
            outputs: output_paths,
            done_path: self.tkr_tmp_dir.join("_done").clone(),
            error_path: self.tkr_tmp_dir.join("_error").clone(),
            ..Default::default()
        })
    }

    async fn start_single_job(
        &self,
        task: &TaskPlan,
        worker_args_path: &Path,
        script_path: &Path,
    ) -> miette::Result<String> {
        let request = ResourceRequest::try_from(task.resources.clone())?;
        let hpc_environment: HashMap<String, String> =
            serde_json::from_value(task.environment.clone().into_iter().collect())
                .into_diagnostic()
                .wrap_err("Invalid HPC environment specification")?;
        let command = worker_invocation(&self.worker_commands, &task.worker_name, worker_args_path);
        let mut job_spec = JobSpec {
            name: format!("tierkreis-{}", task.workflow_run_id),
            command,
            walltime: request.walltime.unwrap_or_else(|| "01:00:00".to_string()),
            resources: request.resources,
            environment: hpc_environment,
            queue: request.queue,
            account: request.account,
            modules: request.modules,
            user: request.user,
            mpi: request.mpi,
            container: request.container,
            extra_scheduler_args: request.extra_scheduler_args,
            ..Default::default()
        };
        let context = tracing::Span::current().context();
        inject_trace_context(&context, &mut EnvironmentCarrier(&mut job_spec.environment));
        let result = self.scheduler.submit(job_spec, script_path).await?;
        Ok(result)
    }

    async fn is_job_active(&self, job_id: &str) -> miette::Result<String> {
        match self.scheduler.check(&[job_id]).await?.get(job_id) {
            Some(
                SchedulerStatus::Queued | SchedulerStatus::Complete | SchedulerStatus::Running,
            ) => Ok(job_id.to_string()),
            Some(SchedulerStatus::Cancelled) => Err(miette!("Job was cancelled")),
            Some(SchedulerStatus::Error { message }) => Err(miette!("Job errored: {}", message)),
            _ => Err(miette!("Job not active")),
        }
    }
}

impl<T: SchedulerWrapper + 'static> Executor for HPCExecutor<T> {
    // TODO: How to make sure this is run on the compute node?
    fn workers(&self) -> BoxFuture<'_, miette::Result<Vec<WorkerSpec>>> {
        async move {
            let task = tokio::task::spawn_blocking(|| {
                let re = Regex::new(r"tkr-.*-worker")
                    .into_diagnostic()
                    .wrap_err("Failed to compile Worker name regex")?;
                let paths = which_re(&re)
                    .into_diagnostic()
                    .wrap_err("Failed to search for Worker binaries")?;
                paths
                    .map(|path| {
                        let worker_name = path
                            .file_name()
                            .and_then(|name| name.to_str())
                            .ok_or_else(|| {
                                miette!(
                                    "Worker binary path has no valid UTF-8 file name: {}",
                                    path.display()
                                )
                            })?;
                        Ok(WorkerSpec {
                            worker_name: worker_name.to_owned(),
                        })
                    })
                    .collect::<miette::Result<Vec<_>>>()
            });
            task.await.into_diagnostic()?
        }
        .boxed()
    }

    fn execute(&self, task_plans: Vec<TaskPlan>) -> BoxFuture<'_, miette::Result<()>> {
        async move {
            let mut task_sender = self.task_sender.clone();

            for task_plan in task_plans {
                let tmp_paths = self.reserve_tmp_paths(2)?;
                let worker_args_path = &tmp_paths[0];
                let script_path = &tmp_paths[1];
                let (outputs, output_paths) = self.build_outputs(task_plan.outputs.clone()).await?;
                let worker_args_file = std::fs::File::create(worker_args_path).into_diagnostic()?;

                // If we were given a handle to a previously submitted  job
                // and it's still active, reattach to it instead of resubmitting.
                // TODO: make the retry logic more explicit
                let job_id = if let Some(job_id) = &task_plan.task_handle {
                    self.is_job_active(job_id).await?
                } else {
                    let worker_args = self
                        .build_worker_call_args(&task_plan, output_paths)
                        .await?;
                    serde_json::to_writer(worker_args_file, &worker_args).into_diagnostic()?;
                    self.start_single_job(&task_plan, worker_args_path, script_path)
                        .await?
                };

                task_sender
                    .send(BackgroundTaskPlan {
                        workflow_run_id: task_plan.workflow_run_id,
                        attempt: task_plan.attempt,
                        loc: task_plan.loc,
                        job_id,
                        outputs,
                        output_storage_name: self.output_storage_name.clone(),
                        _worker_args: worker_args_path.clone(),
                    })
                    .await
                    .into_diagnostic()?;
            }

            Ok(())
        }
        .boxed()
    }

    fn listen(&self) -> miette::Result<BoxStream<'static, RuntimeEvent>> {
        let channel = {
            let mut receiver = self
                .event_receiver
                .try_lock()
                .map_err(|err| miette!("Failed to listen: {}", err))?;

            receiver.take().ok_or_else(|| {
                miette!("Failed to listen: HPCExecutor is already being listened to.")
            })?
        };
        Ok(channel.boxed())
    }

    fn cancel(
        &self,
        workflow_run_id: Uuid,
        attempt: u32,
        task_locations: Vec<Location>,
    ) -> BoxFuture<'_, miette::Result<()>> {
        let mut cancel_sender = self.cancel_sender.clone();
        let fut = async move {
            for task_location in task_locations {
                cancel_sender
                    .send((workflow_run_id, attempt, task_location))
                    .await
                    .into_diagnostic()
                    .wrap_err("Failed to enqueue background task.")?;
            }
            Ok(())
        };
        fut.boxed()
    }
}

#[cfg(test)]
mod tests {

    use std::assert_matches;

    use futures::StreamExt;
    use serde_json::json;

    use super::*;
    use crate::{
        asset_storage::{FileAssetStorage, assert_registry_contains_values, test_storage_registry},
        event::{NodeStatus, WorkflowRunEvent},
        executor::{HPCExecutor, PbsWrapper, SlurmWrapper},
    };

    #[test]
    fn resource_request_ignores_unknown_keys() -> miette::Result<()> {
        let request = ResourceRequest::try_from(HashMap::from([
            ("cpu".to_string(), json!(4)),
            ("future_resource".to_string(), json!(true)),
            (
                "extras".to_string(),
                json!({"flags": ["--verbose"], "future": ["ignored"]}),
            ),
        ]))?;

        assert_eq!(request.resources.cores_per_node, Some(4));
        assert_eq!(request.extra_scheduler_args["--verbose"], None);
        Ok(())
    }

    async fn execute_hpc_with_scheduler<T: SchedulerWrapper + 'static>(
        scheduler: T,
        extra_scheduler_args: Option<serde_json::Value>,
        expected_output: serde_json::Value,
    ) -> miette::Result<()> {
        let checkpoints_path = std::env::var_os("HOME")
            .map(PathBuf::from)
            .ok_or_else(|| miette!("HOME is not set"))?
            .join(".tierkreis/slrm"); // Mimic scratch on a cluster
        let file_storage = FileAssetStorage::try_new(&checkpoints_path)?;
        let (registry, input_sets, _dir) =
            test_storage_registry(vec![json!({"value": "Test"})], vec![]).await;
        let mut registry = Arc::try_unwrap(registry)
            .unwrap_or_else(|_| panic!("test registry should have one owner"));
        registry.insert("checkpoints".to_string(), Box::new(file_storage));
        let registry = Arc::new(registry);
        let mut outputs = HashSet::new();
        outputs.insert("value".to_string());
        let mut task_resources = HashMap::new();
        task_resources.insert("nodes".to_string(), 2.into());
        task_resources.insert("mpi".to_string(), json!({"max_proc_per_node": "1"}));
        if let Some(extra_scheduler_args) = extra_scheduler_args {
            task_resources.insert("extra_scheduler_args".to_string(), extra_scheduler_args);
        }
        let task_plans = vec![TaskPlan {
            loc: Location::default(),
            worker_name: "mpi_worker".to_string(),
            task_name: "mpi_rank_info_with_input".to_string(),
            outputs,
            inputs: input_sets[0].clone(),
            resources: task_resources,
            environment: HashMap::from([
                ("OMPI_ALLOW_RUN_AS_ROOT".to_string(), json!("1")),
                ("OMPI_ALLOW_RUN_AS_ROOT_CONFIRM".to_string(), json!("1")),
            ]),
            ..Default::default()
        }];
        let resources = HPCResourceSpec {
            nodes: 2,
            cores_per_node: Some(1),
            memory_per_node_gb: Some(1),
            gpus_per_node: Some(0),
            qpus: None,
            gres: None,
        };
        let executor = HPCExecutor::try_new(
            &registry,
            "checkpoints",
            "checkpoints",
            Arc::new(scheduler),
            resources,
            Duration::from_secs(1),
            HashMap::from([(
                "mpi_worker".to_string(),
                ["uv", "run", "/mpi_worker/main.py"]
                    .into_iter()
                    .map(str::to_owned)
                    .collect(),
            )]),
        )?;

        let stream = executor.listen()?;
        executor.execute(task_plans).await?;

        let events = stream.take(3).collect::<Vec<_>>().await;
        dbg!(&events);
        assert_eq!(events.len(), 3);
        assert_matches!(
            events[0],
            RuntimeEvent::WorkflowRun {
                event: WorkflowRunEvent::NodeEvents(ref node_events),
                ..
            } if matches!(node_events[0].status, NodeStatus::Queued {..})
        );
        assert_matches!(
            events[1],
            RuntimeEvent::WorkflowRun {
                event: WorkflowRunEvent::NodeEvents(ref node_events),
                ..
            } if matches!(node_events[0].status, NodeStatus::Running {..})
        );
        assert_registry_contains_values(
            &registry,
            "checkpoints",
            &events[2].clone().outputs()[0],
            expected_output,
        )
        .await;

        Ok(())
    }

    // Test that we can launch a task and listen for
    // errors when they occur
    #[tokio::test]
    #[ignore = "Requires a local SLURM setup. To run the test, ensure infra/slurm_local is running. Then run the test from outside the Docker container."]
    async fn execute_hpc_slurm() -> miette::Result<()> {
        execute_hpc_with_scheduler(
            SlurmWrapper::local(),
            None,
            json!({"value": "Rank 0 out of 2 on c1 with value Test.\nRank 1 out of 2 on c2 with value Test."}),
        )
        .await
    }

    // Test that we can launch a task and listen for
    // errors when they occur
    #[tokio::test]
    #[ignore = "Requires a local PBS setup. To run the test, ensure infra/pbs_local is running. Then run the test from outside the Docker container."]
    async fn execute_hpc_pbs() -> miette::Result<()> {
        execute_hpc_with_scheduler(
            PbsWrapper::local(),
            Some(json!({"-l": "place=scatter"})), // by default PBS reuses the same node
            json!({"value": "Rank 0 out of 2 on p1 with value Test.\nRank 1 out of 2 on p2 with value Test."}),
        )
        .await
    }
}
