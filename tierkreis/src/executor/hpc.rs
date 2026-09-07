//! Event-based execution of subprocess-compatible workers on a batch scheduler.

pub mod slurm;
pub mod spec;

use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{Arc, Mutex},
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
use tempfile::NamedTempFile;
use tokio::task::AbortHandle;
use uuid::Uuid;
use which::which_re;

use crate::{
    asset_storage::{AssetSpec, AssetStorageRegistry, reserve_asset_specs, transfer_assets},
    event::{
        EventReceiver, EventSender, RuntimeEvent, send_cancelled, send_complete, send_error,
        send_queued, send_running,
    },
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

struct BackgroundTaskPlan {
    workflow_run_id: Uuid,
    attempt: u32,
    loc: Location,
    job_spec: JobSpec,
    outputs: HashMap<String, AssetSpec>,
    output_storage_name: String,
    worker_args: NamedTempFile,
    done_file: NamedTempFile,
    error_file: NamedTempFile,
    task_handle: Option<TaskHandle>,
}

struct BackgroundTask {
    workflow_run_id: Uuid,
    attempt: u32,
    loc: Location,
    outputs: HashMap<String, AssetSpec>,
    output_storage_name: String,
    result: miette::Result<()>,
    _worker_args: NamedTempFile,
    _done_file: NamedTempFile,
    _error_file: NamedTempFile,
}

type Key = (Uuid, u32, Location);
type Running = FuturesUnordered<BoxFuture<'static, BackgroundTask>>;

async fn process_tasks(
    scheduler: Arc<dyn SchedulerWrapper>,
    mut tasks: mpsc::Receiver<BackgroundTaskPlan>,
    mut cancels: mpsc::Receiver<Key>,
    mut events: EventSender,
    registry: AssetStorageRegistry,
) {
    let mut running: Running = FuturesUnordered::new();
    let mut jobs: HashMap<Key, String> = HashMap::new();
    loop {
        tokio::select! {
            Some((workflow_run_id, attempt, loc)) = cancels.next() => {
                if let Some(job_id) = jobs.remove(&(workflow_run_id, attempt, loc.clone())) {
                    if let Err(error) = scheduler.cancel(job_id).await {
                        send_error(&mut events, workflow_run_id, attempt, loc, &error).await.ok();
                    } else {
                        send_cancelled(&mut events, workflow_run_id, attempt, loc).await.ok();
                    }
                }
            }
            Some(task) = tasks.next() => {
                let key = (task.workflow_run_id, task.attempt, task.loc.clone());
                let result = if let Some(job_id) = task.task_handle.clone() {
                    Ok(job_id)
                } else {
                    scheduler.submit(task.job_spec.clone()).await
                };
                match result {
                    Ok(job_id) => {
                        send_queued(&mut events, task.workflow_run_id, task.attempt, task.loc.clone(), Some(job_id.clone())).await.ok();
                        send_running(&mut events, task.workflow_run_id, task.attempt, task.loc.clone()).await.ok();
                        jobs.insert(key, job_id.clone());
                        let scheduler = scheduler.clone();
                        running.push(async move {
                            let result = scheduler.wait(job_id.clone()).await;
                            BackgroundTask { workflow_run_id: task.workflow_run_id, attempt: task.attempt, loc: task.loc, outputs: task.outputs, output_storage_name: task.output_storage_name, result, _worker_args: task.worker_args, _done_file: task.done_file, _error_file: task.error_file }
                        }.boxed());
                    }
                    Err(error) => { send_error(&mut events, task.workflow_run_id, task.attempt, task.loc, &error).await.ok(); }
                }
            }
            Some(task) = running.next() => {
                jobs.remove(&(task.workflow_run_id, task.attempt, task.loc.clone()));
                match task.result {
                    Ok(()) => match transfer_assets(&registry, &task.output_storage_name, &task.outputs).await {
                        Ok(outputs) => send_complete(&mut events, task.workflow_run_id, task.attempt, vec![task.loc], vec![outputs]).await.ok(),
                        Err(error) => send_error(&mut events, task.workflow_run_id, task.attempt, task.loc, &error).await.ok(),
                    },
                    Err(error) => send_error(&mut events, task.workflow_run_id, task.attempt, task.loc, &error).await.ok(),
                };
            }
            else => break,
        }
    }
}

/// Event-based executor for subprocess-compatible workers.
pub struct HpcExecutor {
    _scheduler: Arc<dyn SchedulerWrapper>,
    task_sender: mpsc::Sender<BackgroundTaskPlan>,
    cancel_sender: mpsc::Sender<Key>,
    event_receiver: Mutex<Option<EventReceiver>>,
    background_abort_handle: AbortHandle,
    hpc_storage_name: String,
    output_storage_name: String,
    asset_storage_registry: AssetStorageRegistry,
    /// Available HPC resources for this executor.
    pub max_resources: HPCResourceSpec,
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
        registry: &AssetStorageRegistry,
        hpc_storage_name: &str,
        output_storage_name: &str,
        scheduler: Arc<dyn SchedulerWrapper>,
        max_resources: HPCResourceSpec,
    ) -> miette::Result<Self> {
        let storage = registry.read().await;
        if !storage.contains_key(hpc_storage_name) {
            return Err(miette!("hpc_storage_name not in registry"));
        }
        if !storage.contains_key(output_storage_name) {
            return Err(miette!("output_storage_name not in registry"));
        }
        drop(storage);
        let (task_sender, tasks) = mpsc::channel(64);
        let (event_sender, event_receiver) = mpsc::channel(64);
        let (cancel_sender, cancels) = mpsc::channel(64);
        let background = tokio::spawn(process_tasks(
            scheduler.clone(),
            tasks,
            cancels,
            event_sender,
            Arc::clone(registry),
        ));
        Ok(Self {
            _scheduler: scheduler,
            task_sender,
            cancel_sender,
            event_receiver: Mutex::new(Some(event_receiver)),
            background_abort_handle: background.abort_handle(),
            hpc_storage_name: hpc_storage_name.into(),
            output_storage_name: output_storage_name.into(),
            asset_storage_registry: Arc::clone(registry),
            max_resources,
        })
    }

    async fn build_task(&self, task: TaskPlan) -> miette::Result<BackgroundTaskPlan> {
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
        let output_specs = reserve_asset_specs(
            &self.asset_storage_registry,
            &self.hpc_storage_name,
            task.outputs.len(),
        )
        .await?;
        let outputs: HashMap<_, _> = task.outputs.into_iter().zip(output_specs).collect();
        let output_paths = outputs
            .iter()
            .map(|(name, spec)| Ok((name.clone(), spec.path()?)))
            .collect::<miette::Result<HashMap<_, _>>>()?;
        let worker_args = NamedTempFile::new().into_diagnostic()?;
        let done_file = NamedTempFile::new().into_diagnostic()?;
        let error_file = NamedTempFile::new().into_diagnostic()?;
        serde_json::to_writer(
            &worker_args,
            &WorkerCallArgs {
                function_name: task.task_name.clone(),
                inputs: input_paths,
                outputs: output_paths,
                done_path: done_file.path().into(),
                error_path: error_file.path().into(),
                ..Default::default()
            },
        )
        .into_diagnostic()?;
        let command = format!(
            "tkr-{} {}",
            task.worker_name.replace('_', "-"),
            worker_args.path().display()
        );
        let hpc_resources = serde_json::from_value(task.resources.into_iter().collect())
            .into_diagnostic()
            .wrap_err("Invalid HPC resource specification")?;
        let job_spec = JobSpec {
            name: format!("tierkreis-{}", task.workflow_run_id),
            command,
            walltime: "01:00:00".to_string(),
            resources: hpc_resources,
            ..Default::default()
        };
        Ok(BackgroundTaskPlan {
            workflow_run_id: task.workflow_run_id,
            attempt: task.attempt,
            loc: task.loc,
            job_spec,
            outputs,
            output_storage_name: task
                .output_storage_name
                .unwrap_or_else(|| self.output_storage_name.clone()),
            worker_args,
            done_file,
            error_file,
            task_handle: task.task_handle,
        })
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
            let mut sender = self.task_sender.clone();
            for task_plan in task_plans {
                sender
                    .send(self.build_task(task_plan).await?)
                    .await
                    .into_diagnostic()
                    .wrap_err("Failed to enqueue background task.")?;
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
