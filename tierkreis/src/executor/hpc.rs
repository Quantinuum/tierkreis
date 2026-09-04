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
use miette::{IntoDiagnostic, miette};
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
    executor::hpc::spec::{JobSpec, SchedulerWrapper},
    executor::interface::{Executor, TaskHandle, TaskPlan, WorkerSpec},
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
}

impl Drop for HpcExecutor {
    fn drop(&mut self) {
        self.background_abort_handle.abort();
    }
}

impl HpcExecutor {
    /// Create an HPC executor using shared file-backed storage.
    pub async fn try_new(
        registry: &AssetStorageRegistry,
        hpc_storage_name: &str,
        output_storage_name: &str,
        scheduler: Arc<dyn SchedulerWrapper>,
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
        // TODO: Load Resources or put spec in TaskPlan
        let job_spec  = JobSpec {
                name: format!("tierkreis-{}", task.workflow_run_id),
                command,
                walltime: "01:00:00".to_string(),
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
    fn workers(&self) -> BoxFuture<'_, miette::Result<Vec<WorkerSpec>>> {
        // TODO: How to make sure this is run on the compute node?
        async move {
            let regex = regex::Regex::new(r"tkr-.*-worker").into_diagnostic()?;
            let paths = tokio::task::spawn_blocking(move || {
                which_re(&regex)
                    .into_diagnostic()
                    .map(|paths| paths.collect::<Vec<_>>())
            })
            .await
            .into_diagnostic()??;
            Ok(paths
                .into_iter()
                .filter_map(|path| {
                    path.file_name()
                        .and_then(|name| name.to_str())
                        .map(|name| WorkerSpec {
                            worker_name: name.trim_start_matches("tkr-").replace('-', "_"),
                        })
                })
                .collect())
        }
        .boxed()
    }
    fn execute(&self, tasks: Vec<TaskPlan>) -> BoxFuture<'_, miette::Result<()>> {
        async move {
            let mut sender = self.task_sender.clone();
            for task in tasks {
                sender
                    .send(self.build_task(task).await?)
                    .await
                    .into_diagnostic()?;
            }
            Ok(())
        }
        .boxed()
    }
    fn listen(&self) -> miette::Result<BoxStream<'static, RuntimeEvent>> {
        let mut receiver = self
            .event_receiver
            .try_lock()
            .map_err(|error| miette!("Failed to listen: {error}"))?;
        receiver
            .take()
            .map(|receiver| receiver.boxed())
            .ok_or_else(|| miette!("Executor is already being listened to"))
    }
    fn cancel(
        &self,
        workflow_run_id: Uuid,
        attempt: u32,
        locs: Vec<Location>,
    ) -> BoxFuture<'_, miette::Result<()>> {
        async move {
            let mut sender = self.cancel_sender.clone();
            for loc in locs {
                sender
                    .send((workflow_run_id, attempt, loc))
                    .await
                    .into_diagnostic()?;
            }
            Ok(())
        }
        .boxed()
    }
}
