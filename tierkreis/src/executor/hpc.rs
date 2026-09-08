//! Event-based execution of subprocess-compatible workers on a batch scheduler.

pub mod slurm;
pub mod spec;

use std::{
    collections::{HashMap, HashSet},
    env::home_dir,
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
        hpc::spec::{HPCResourceSpec, JobSpec, SchedulerStatus, SchedulerWrapper},
        interface::{Executor, TaskHandle, TaskPlan, WorkerSpec},
    },
    location::Location,
};

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

struct EnvironmentCarrier<'a>(&'a mut HashMap<String, String>);

impl opentelemetry::propagation::Injector for EnvironmentCarrier<'_> {
    fn set(&mut self, key: &str, value: String) {
        self.0.insert(key.to_uppercase().replace('-', "_"), value);
    }
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

enum InternalJobMonitoringStatus {
    Watching {
        job_id: String,
        task: BackgroundTask,
        status: Option<SchedulerStatus>,
    },
    Cancelling,
}

type Key = (Uuid, u32, Location);
type TaskReceiver = mpsc::Receiver<BackgroundTaskPlan>;
type CancelReceiver = mpsc::Receiver<Key>;
type JobHandles = HashMap<Key, InternalJobMonitoringStatus>;
type OutputSpecs = (HashMap<String, AssetSpec>, HashMap<String, PathBuf>);

// Identical to nexus
#[instrument(skip_all, err)]
async fn process_cancelled_task(
    scheduler: Arc<dyn SchedulerWrapper>,
    job_handles: &mut JobHandles,
    workflow_run_id: Uuid,
    attempt: u32,
    loc: Location,
) -> miette::Result<()> {
    let key = (workflow_run_id, attempt, loc.clone());
    let handle = job_handles.remove(&key);
    match handle {
        // We know the job id, cancel the job.
        Some(InternalJobMonitoringStatus::Watching { job_id, .. }) => {
            tracing::info!("Cancelling job for node: {key:?}");
            scheduler.cancel(job_id.clone()).await?;
        }
        // We are already planning to cancel the job if we see this node.
        Some(InternalJobMonitoringStatus::Cancelling) => {}
        // Register that the node needs to be cancelled if we see it.
        None => {
            warn!("Got cancel request before registering job_id for node: {key:?}");
            job_handles.insert(key, InternalJobMonitoringStatus::Cancelling);
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
                    send_complete(
                        event_sender,
                        workflow_run_id,
                        attempt,
                        vec![loc],
                        vec![outputs],
                    )
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
async fn monitor_task(
    scheduler: Arc<dyn SchedulerWrapper>,
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
        Some(InternalJobMonitoringStatus::Cancelling)
    );
    job_handles.insert(
        key,
        InternalJobMonitoringStatus::Watching {
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

async fn check_jobs(
    scheduler: &Arc<dyn SchedulerWrapper>,
    event_sender: &mut EventSender,
    job_handles: &mut JobHandles,
    asset_storage_registry: &AssetStorageRegistry,
) -> miette::Result<()> {
    let jobs: Vec<_> = job_handles
        .iter()
        .filter_map(|(key, status)| match status {
            InternalJobMonitoringStatus::Watching { job_id, .. } => {
                Some((key.clone(), job_id.clone()))
            }
            InternalJobMonitoringStatus::Cancelling => None,
        })
        .collect();
    if jobs.is_empty() {
        return Ok(());
    }

    let statuses = scheduler
        .check(jobs.iter().map(|(_, job_id)| job_id.clone()).collect())
        .await?;
    for (key, job_id) in jobs {
        let Some(status) = statuses.get(&job_id).cloned() else {
            continue;
        };
        match status {
            SchedulerStatus::Queued | SchedulerStatus::Running => {
                let InternalJobMonitoringStatus::Watching {
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
                let Some(InternalJobMonitoringStatus::Watching { task, .. }) =
                    job_handles.remove(&key)
                else {
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

async fn process_tasks(
    scheduler: Arc<dyn SchedulerWrapper>,
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
pub struct HPCExecutor {
    scheduler: Arc<dyn SchedulerWrapper>,
    task_sender: mpsc::Sender<BackgroundTaskPlan>,
    cancel_sender: mpsc::Sender<Key>,
    event_receiver: Mutex<Option<EventReceiver>>,
    background_abort_handle: AbortHandle,
    hpc_storage_name: String,
    output_storage_name: String,
    asset_storage_registry: AssetStorageRegistry,
    /// Available HPC resources for this executor.
    pub max_resources: HPCResourceSpec,
    worker_command: Option<String>,
    tkr_tmp_dir: PathBuf,
}

impl Drop for HPCExecutor {
    fn drop(&mut self) {
        self.background_abort_handle.abort();
    }
}

impl HPCExecutor {
    /// Create an HPC executor using shared file-backed storage./
    ///
    /// # Errors
    ///
    /// If the specified storage names are not present in the registry.
    pub async fn try_new(
        asset_storage_registry: &AssetStorageRegistry,
        hpc_storage_name: &str,
        output_storage_name: &str,
        scheduler: Arc<dyn SchedulerWrapper>,
        max_resources: HPCResourceSpec,
        poll_interval: Duration,
    ) -> miette::Result<Self> {
        let storage = asset_storage_registry.read().await;
        if !storage.contains_key(hpc_storage_name) {
            return Err(miette!("hpc_storage_name not in registry"));
        }
        if !storage.contains_key(output_storage_name) {
            return Err(miette!("output_storage_name not in registry"));
        }
        let tierkreis_dir = home_dir()
            .unwrap_or_else(|| "/tmp".into())
            .join(".tierkreis/tmp");
        std::fs::create_dir_all(&tierkreis_dir)
            .into_diagnostic()
            .wrap_err_with(|| {
                format!(
                    "Failed to create Tierkreis temporary directory at {}",
                    tierkreis_dir.display()
                )
            })?;

        drop(storage);
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
            worker_command: None,
            tkr_tmp_dir: tierkreis_dir,
        })
    }

    /// Override the command used to invoke workers.
    #[must_use]
    #[cfg(test)]
    pub fn with_worker_command(mut self, worker_command: impl Into<String>) -> Self {
        self.worker_command = Some(worker_command.into());
        self
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
        let worker_command = self
            .worker_command
            .clone()
            .unwrap_or_else(|| format!("tkr-{}", task.worker_name.replace('_', "-")));
        let command = format!("{worker_command} {}", worker_args_path.display());
        let hpc_resources = serde_json::from_value(task.resources.clone().into_iter().collect())
            .into_diagnostic()
            .wrap_err("Invalid HPC resource specification")?;
        let hpc_environment =
            serde_json::from_value(task.environment.clone().into_iter().collect())
                .into_diagnostic()
                .wrap_err("Invalid HPC environment specification")?;
        // TODO get other JobSpec Related fields from the task.resources
        let mut job_spec = JobSpec {
            name: format!("tierkreis-{}", task.workflow_run_id),
            command,
            walltime: "01:00:00".to_string(),
            resources: hpc_resources,
            environment: hpc_environment,
            ..Default::default()
        };
        let context = tracing::Span::current().context();
        opentelemetry::global::get_text_map_propagator(|propagator| {
            propagator.inject_context(&context, &mut EnvironmentCarrier(&mut job_spec.environment));
        });
        let result = self.scheduler.submit(job_spec, script_path).await?;
        Ok(result)
    }

    async fn is_job_active(&self, job_id: String) -> miette::Result<String> {
        match self
            .scheduler
            .check(vec![job_id.clone()])
            .await?
            .get(&job_id)
        {
            Some(
                SchedulerStatus::Queued | SchedulerStatus::Complete | SchedulerStatus::Running,
            ) => Ok(job_id),
            _ => Err(miette!("Job not active")),
        }
    }
}

impl Executor for HPCExecutor {
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
                Ok(paths
                    .map(|path| WorkerSpec {
                        worker_name: path.file_name().unwrap().to_str().unwrap().to_string(),
                    })
                    .collect())
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
                let job_id = if let Some(job_id) = &task_plan.task_handle {
                    self.is_job_active(job_id.clone()).await?
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

    use futures::StreamExt;
    use serde_json::json;

    use super::*;
    use crate::{
        asset_storage::{FileAssetStorage, assert_registry_contains_values, test_storage_registry},
        event::{NodeEvent, NodeStatus, WorkflowRunEvent},
        executor::{HPCExecutor, SlurmWrapper},
    };
    // Test that we can launch a task and listen for
    // errors when they occur
    #[tokio::test]
    #[ignore = "Requires a local SLURM setup"]
    async fn execute_hpc() -> miette::Result<()> {
        let checkpoints_path = std::env::var_os("HOME")
            .map(PathBuf::from)
            .ok_or_else(|| miette!("HOME is not set"))?
            .join(".tierkreis/slrm");
        let file_storage = FileAssetStorage::new(&checkpoints_path);
        let (registry, input_sets, _dir) =
            test_storage_registry(vec![json!({"value": "Test"})], vec![]).await;
        registry
            .write()
            .await
            .insert("checkpoints".to_string(), Box::new(file_storage));
        let mut outputs = HashSet::new();
        outputs.insert("value".to_string());
        let mut task_resources = HashMap::new();
        task_resources.insert("nodes".to_string(), 2.into());
        let task_plans = vec![TaskPlan {
            loc: Location::default(),
            worker_name: "mpi_worker".to_string(),
            task_name: "mpi_rank_info_with_input".to_string(),
            outputs,
            inputs: input_sets[0].clone(),
            resources: task_resources,
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
        let scheduler = SlurmWrapper::local();

        let executor = HPCExecutor::try_new(
            &registry,
            "checkpoints",
            "checkpoints",
            Arc::new(scheduler),
            resources,
            Duration::from_secs(1),
        )
        .await?
        .with_worker_command("mpiexec --allow-run-as-root uv run /mpi_worker/main.py");
        // TODO: enable mpi environment

        let stream = executor.listen()?;
        executor.execute(task_plans).await?;

        let events = stream.take(3).collect::<Vec<_>>().await;
        dbg!(&events);
        assert_eq!(events.len(), 3);
        assert!(matches!(
            events[0],
            RuntimeEvent::WorkflowRun {
                event: WorkflowRunEvent::NodeEvent(NodeEvent {
                    status: NodeStatus::Queued { .. },
                    ..
                }),
                ..
            }
        ));
        assert!(matches!(
            events[1],
            RuntimeEvent::WorkflowRun {
                event: WorkflowRunEvent::NodeEvent(NodeEvent {
                    status: NodeStatus::Running { .. },
                    ..
                }),
                ..
            }
        ));
        assert_registry_contains_values(
            &registry,
            "checkpoints",
            &events[2].clone().outputs()[0],
            json!({"value": "Rank 0 out of 2 on c1 with value Test.\nRank 1 out of 2 on c2 with value Test."}),
        ).await;

        Ok(())
    }
}
