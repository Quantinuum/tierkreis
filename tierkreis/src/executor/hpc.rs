//! Event-based execution of subprocess-compatible workers on a batch scheduler.

pub mod slurm;
pub mod spec;

use std::{
    collections::{HashMap, HashSet}, path::{Path, PathBuf}, sync::{Arc, Mutex},
};

use futures::{
    FutureExt, SinkExt, StreamExt,
    channel::mpsc,
    future::BoxFuture,
    stream::{BoxStream, FuturesUnordered},
};
use miette::{Context, IntoDiagnostic, miette};
use regex::Regex;
use serde::{Deserialize, Serialize};
use tokio::task::AbortHandle;
use tracing::{instrument, warn};
use tracing_opentelemetry::OpenTelemetrySpanExt;
use uuid::Uuid;
use which::which_re;

use crate::{
    asset_storage::{AssetSpec, AssetStorageRegistry, reserve_asset_specs, transfer_assets},
    event::{EventReceiver, EventSender, RuntimeEvent, send_complete, send_error, send_queued},
    executor::{
        hpc::spec::{HPCResourceSpec, JobSpec, SchedulerWrapper},
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
    Watching { job_id: String },
    Cancelling,
}

type Key = (Uuid, u32, Location);
type TaskReceiver = mpsc::Receiver<BackgroundTaskPlan>;
type CancelSender = mpsc::Sender<Key>;
type CancelReceiver = mpsc::Receiver<Key>;
type RunningFutures = FuturesUnordered<BoxFuture<'static, (BackgroundTask, miette::Result<()>)>>;
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
        Some(InternalJobMonitoringStatus::Watching { job_id }) => {
            tracing::info!("Cancelling job for node: {key:?}");
            scheduler.cancel(job_id).await?;
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
    result: miette::Result<()>,
) -> miette::Result<()> {
    let loc = background_task.loc;
    let outputs = background_task.outputs;
    let output_storage_name = background_task.output_storage_name;
    let workflow_run_id = background_task.workflow_run_id;
    let attempt = background_task.attempt;

    job_handles.remove(&(workflow_run_id, attempt, loc.clone()));

    // TODO this could have the same states we monitor in the scheduler not a plain result
    match result {
        Ok(()) => {
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
        Err(error) => {
            send_error(event_sender, workflow_run_id, attempt, loc, &error).await?;
        }
    }

    Ok(())
}

#[instrument(skip_all, err)]
async fn monitor_task(
    scheduler: Arc<dyn SchedulerWrapper>,
    event_sender: &EventSender,
    cancel_sender: &mut CancelSender,
    job_handles: &mut JobHandles,
    running: &mut RunningFutures,
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
    // If we are meant to cancel this job, signal this to the cancellation routine.
    if let Some(InternalJobMonitoringStatus::Cancelling) = job_handles.get(&key) {
        cancel_sender.send(key.clone()).await.into_diagnostic()?;
    }
    job_handles.insert(
        key,
        InternalJobMonitoringStatus::Watching {
            job_id: job_id.clone(),
        },
    );
    running.push(
        async move {
            let result = scheduler.wait(job_id).await;
            (
                BackgroundTask {
                    workflow_run_id,
                    attempt,
                    loc,
                    outputs,
                    output_storage_name,
                    _worker_args: worker_args,
                },
                result,
            )
        }
        .boxed(),
    );
    Ok(())
}

async fn process_tasks(
    scheduler: Arc<dyn SchedulerWrapper>,
    mut task_receiver: TaskReceiver,
    mut cancel_sender: CancelSender,
    mut cancel_receiver: CancelReceiver,
    mut event_sender: EventSender,
    asset_storage_registry: AssetStorageRegistry,
) {
    let mut job_handles: JobHandles = HashMap::new();
    let mut running: RunningFutures = FuturesUnordered::new();

    loop {
        tokio::select! {
            // A task has been canceled
            Some((workflow_run_id, attempt, loc)) = cancel_receiver.next() => {
                process_cancelled_task(scheduler.clone(), &mut job_handles, workflow_run_id, attempt, loc)
                    .await
                    .expect("Failed to cancel task");
            }
            // A task has completed
            Some((task, result)) = running.next() => {
                if let Err(error) = process_finished_task(
                    &mut event_sender,
                    &mut job_handles,
                    &asset_storage_registry,
                    task,
                    result,
                ).await {
                    tracing::error!("Failed to process finished task: {error:?}");
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
                    &mut cancel_sender,
                    &mut job_handles,
                    &mut running,
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
pub struct HpcExecutor {
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
    // TODO: env
}

impl Drop for HpcExecutor {
    fn drop(&mut self) {
        self.background_abort_handle.abort();
    }
}

impl HpcExecutor {
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
    ) -> miette::Result<Self> {
        let storage = asset_storage_registry.read().await;
        if !storage.contains_key(hpc_storage_name) {
            return Err(miette!("hpc_storage_name not in registry"));
        }
        if !storage.contains_key(output_storage_name) {
            return Err(miette!("output_storage_name not in registry"));
        }
        drop(storage);
        let (task_sender, task_receiver) = mpsc::channel(64);
        let (event_sender, event_receiver) = mpsc::channel(64);
        let (cancel_sender, cancel_receiver) = mpsc::channel(64);
        let background = tokio::spawn(process_tasks(
            scheduler.clone(),
            task_receiver,
            cancel_sender.clone(),
            cancel_receiver,
            event_sender,
            Arc::clone(asset_storage_registry),
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
        })
    }

    async fn build_outputs(
        &self,
        output_storage_name: &str,
        outputs: HashSet<String>,
    ) -> miette::Result<OutputSpecs> {
        let output_specs = reserve_asset_specs(
            &self.asset_storage_registry,
            output_storage_name,
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
        // TODO: Get rid of the hardcoded done and error paths
        Ok(WorkerCallArgs {
            function_name: task.task_name.clone(),
            inputs: input_paths,
            outputs: output_paths,
            done_path: std::path::Path::new("_done").to_path_buf(),
            error_path: std::path::Path::new("_error").to_path_buf(),
            ..Default::default()
        })
    }

    async fn start_single_job(
        &self,
        task: &TaskPlan,
        worker_args_path: &Path,
        script_path: &Path,
    ) -> miette::Result<String> {
        let command = format!(
            "tkr-{} {}",
            task.worker_name.replace('_', "-"),
            worker_args_path.display(),
        );
        let hpc_resources = serde_json::from_value(task.resources.clone().into_iter().collect())
            .into_diagnostic()
            .wrap_err("Invalid HPC resource specification")?;
        // TODO get other JobSpec Related fields from the task.resources
        let mut job_spec = JobSpec {
            name: format!("tierkreis-{}", task.workflow_run_id),
            command,
            walltime: "01:00:00".to_string(),
            resources: hpc_resources,
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
        self.scheduler.check(job_id.clone()).await?;
        Ok(job_id)
    }
}

impl Executor for HpcExecutor {
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
                let (outputs, output_paths) = self
                    .build_outputs(&self.output_storage_name, task_plan.outputs.clone())
                    .await?;
                // TODO somehow we need to get back the paths after a crash instead of generating new ones
                // Could use the task handle but seems a bit fragile
                let tmp_assets: &Vec<AssetSpec> =
                    &reserve_asset_specs(&self.asset_storage_registry, &self.hpc_storage_name, 2)
                        .await?;
                let worker_args_path = tmp_assets[0].path()?;
                let worker_args_file =
                    std::fs::File::create(&worker_args_path).into_diagnostic()?;
                let script_path = tmp_assets[1].path()?;

                // If we were given a handle to a previously submitted  job
                // and it's still active, reattach to it instead of resubmitting.
                let job_id = if let Some(job_id) = &task_plan.task_handle {
                    self.is_job_active(job_id.clone()).await?
                } else {
                    let worker_args = self
                        .build_worker_call_args(&task_plan, output_paths)
                        .await?;
                    serde_json::to_writer(worker_args_file, &worker_args).into_diagnostic()?;
                    self.start_single_job(&task_plan, &worker_args_path, &script_path)
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
                        _worker_args: worker_args_path,
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
