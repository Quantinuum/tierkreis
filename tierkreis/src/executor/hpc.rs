/*!
This module defines the [`HPCExecutor`] struct which implements [Executor]
by running subprocesses.
*/
use std::{
    collections::{HashMap, HashSet},
    path::PathBuf,
    process::ExitStatus,
    sync::{Arc, Mutex, atomic::AtomicU32}, time::Duration,
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
use tokio::{
    io::AsyncReadExt,
    task::{AbortHandle, JoinHandle},
};
use which::which_re;

use crate::{
    asset_storage::{
        AssetKind, AssetSpec, AssetStorageRegistry, FileAssetStorage, file, reserve_asset_specs, transfer_assets
    },
    event::{Event, Status, send_cancelled, send_complete, send_error, send_running},
    executor::{interface::{Executor, TaskPlan, WorkerSpec}, slurm::{parse_and_extract_job_id, poll_slurm_status, submit_job, write_jobscript}},
};

/// [`HPCResourceSpec`] determines what Resources should be available to the
/// [`HPCExecutor`] or what is requested as part of a [`TaskPlan`].
#[derive(Clone, Debug, PartialEq, Default)]
pub struct HPCResourceSpec {}

/// [`HPCEnvironmentSpec`] determines the default execution environment of
/// [`HPCExecutor`] or what is requested as part of a [`TaskPlan`].
#[derive(Clone, Debug, PartialEq, Default)]
pub struct HPCEnvironmentSpec {}

pub struct BackgroundTaskPlan {
    id: u32,
    worker_name: String,
    output_storage_name: String,
    pub worker_args: PathBuf,
    pub script_path: PathBuf,
    done_file: PathBuf,
    outputs: HashMap<String, AssetSpec>,
    resources: HPCResourceSpec, // Unused for now -> used to generate job script    
    environment: HPCEnvironmentSpec,
}

struct BackgroundTask {
    id: u32,
    hpc_id: String, // See if we can use this as id
    output_storage_name: String,
    exit_status: Result<ExitStatus, miette::Error>,
    outputs: HashMap<String, AssetSpec>,
    stderr: JoinHandle<String>,

    // TODO: Do I need these?
    // Handles to temporary files to prevent deletion
    // until the task is complete.
    _worker_args: PathBuf,
    _done_file: PathBuf,
}

type TaskSender = mpsc::Sender<BackgroundTaskPlan>;
type TaskReceiver = mpsc::Receiver<BackgroundTaskPlan>;
type EventSender = mpsc::Sender<Event>;
type EventReceiver = mpsc::Receiver<Event>;
type CancelSender = mpsc::Sender<u32>;
type CancelReceiver = mpsc::Receiver<u32>;

type RunningFutures = FuturesUnordered<JoinHandle<BackgroundTask>>;

async fn process_cancelled_task(
    event_sender: &mut EventSender,
    abort_handles: &mut HashMap<u32, AbortHandle>,
    id: u32,
) -> miette::Result<()> {
    let handle = abort_handles.remove(&id);
    if let Some(handle) = handle {
        handle.abort();
        send_cancelled(event_sender, id).await?;
    }
    Ok(())
}

async fn process_finished_task(
    event_sender: &mut EventSender,
    abort_handles: &mut HashMap<u32, AbortHandle>,
    asset_storage_registry: &AssetStorageRegistry,
    background_task: BackgroundTask,
) -> miette::Result<()> {
    let id = background_task.id;
    let outputs = background_task.outputs;
    let output_storage_name = background_task.output_storage_name;
    let exit_status = background_task.exit_status;

    abort_handles.remove(&id);
    match exit_status {
        Ok(status) => {
            if status.success() {
                let outputs =
                    transfer_assets(asset_storage_registry, &output_storage_name, &outputs);
                match outputs {
                    Ok(outputs) => send_complete(event_sender, id, outputs).await?,
                    Err(err) => send_error(event_sender, id, &err).await?,
                }
            } else {
                let stderr = background_task.stderr.await.ok();
                event_sender
                    .send(Event {
                        id,
                        status: Status::Error {
                            error: format!("Subprocess failed with exit code: {status}"),
                            detail: stderr,
                        },
                    })
                    .await
                    .map_err(|err| miette!("Failed to send error event: {err}"))?;
            }
        }
        Err(err) => send_error(event_sender, id, &miette!("Failed to run worker: {err}")).await?,
    }

    Ok(())
}

async fn start_task(
    event_sender: &mut EventSender,
    abort_handles: &mut HashMap<u32, AbortHandle>,
    running: &mut RunningFutures,
    internal_task: BackgroundTaskPlan,
) -> miette::Result<()> {
    let id = internal_task.id;
    send_running(event_sender, id).await?;


    write_jobscript(&internal_task, &internal_task.script_path)?;
    let worker_args = internal_task.worker_args;
    let done_file = internal_task.done_file;
    let res = submit_job(&internal_task.script_path);
    let mut child = match res {
        Ok(child) => child,
        Err(err) => {
            send_error(event_sender, id, &err).await?;
            return Ok(());
        }
    };
    let stderr = read_stderr(&mut child);
    let job_id = parse_and_extract_job_id(&mut child).await.unwrap().unwrap();

    let outputs = internal_task.outputs;
    let output_storage_name = internal_task.output_storage_name;
    dbg!(&job_id);
    let task = tokio::task::spawn(async move {
        let exit_status = poll_slurm_status(&job_id, Duration::new(1, 0), Duration::new(60, 0)).await;
        BackgroundTask {
            id,
            hpc_id: job_id,
            output_storage_name,
            exit_status,
            outputs,
            stderr,
            _worker_args: worker_args,
            _done_file: done_file,
        }
    });

    abort_handles.insert(id, task.abort_handle());
    running.push(task);

    Ok(())
}

async fn process_tasks(
    mut task_receiver: TaskReceiver,
    mut cancel_receiver: CancelReceiver,
    mut event_sender: EventSender,
    asset_storage_registry: AssetStorageRegistry,
) {
    let mut abort_handles: HashMap<u32, AbortHandle> = HashMap::new();
    let mut running: RunningFutures = FuturesUnordered::new();

    loop {
        tokio::select! {
            // A task has been cancelled
            Some(id) = cancel_receiver.next() => {
                process_cancelled_task(&mut event_sender, &mut abort_handles, id)
                    .await
                    .expect("Failed to cancel task");
            }
            // A task has completed
            Some(res) = running.next() => {
                let background_task = match res {
                    Ok(ok) => ok,
                    Err(err) => panic!("Failed to join to future: {err}"),
                };

                process_finished_task(
                    &mut event_sender,
                    &mut abort_handles,
                    &asset_storage_registry,
                    background_task,
                )
                .await
                .expect("Failed to complete task");
            }
            // A task has been submitted
            Some(internal_task) = task_receiver.next() => {
                start_task(
                    &mut event_sender,
                    &mut abort_handles,
                    &mut running,
                    internal_task,
                )
                .await
                .expect("Failed to start task");
            }
        }
    }
}

/// [`SubprocessExecutor`] defines an [Executor] that performs Task Nodes using Worker subprocesses.
pub struct HPCExecutor {
    id_source: AtomicU32,
    task_sender: TaskSender,
    cancel_sender: CancelSender,
    event_receiver: Mutex<Option<EventReceiver>>,

    // The name of the storage that the subprocess will read
    // and write files from. Must be file based.
    subprocess_storage_name: String,
    // The name of the storage to copy outputs to.
    //
    // If this is the same as the subprocess storage name,
    // no copying will occur.
    output_storage_name: String,
    asset_storage_registry: AssetStorageRegistry,
}

type OutputSpecs = (HashMap<String, AssetSpec>, HashMap<String, PathBuf>);

impl HPCExecutor {
    /// Try to create a new [`HPCExecutor`] with an [`AssetStorageRegistry`], a
    /// configured name for an [`AssetStorage`][crate::asset_storage::AssetStorage]
    /// of [`AssetKind::File`][crate::asset_storage::AssetKind::File] where files are written
    /// to for the subprocesses to consume and a configured name for an
    /// [`AssetStorage`][crate::asset_storage::AssetStorage] in the registry that
    /// determines where Assets are saved by default.
    ///
    /// # Errors
    ///
    /// This function will return Err if the specified `subprocess_storage_name` or
    /// `output_storage_name` does not exist inside the [`AssetStorageRegistry`].
    pub fn try_new(
        asset_storage_registry: &AssetStorageRegistry,
        subprocess_storage_name: &str,
        output_storage_name: &str,
    ) -> miette::Result<Self> {
        let asset_storage_registry_lock = asset_storage_registry
            .read()
            .map_err(|err| miette!("Failed to lock AssetStorageRegistry for reading: {err}"))?;
        if let Some(subprocess_storage) = asset_storage_registry_lock.get(subprocess_storage_name) {
            if !matches!(subprocess_storage.kind(), AssetKind::File { .. }) {
                return Err(miette!(
                    "subprocess_storage_name must be of AssetKind::File"
                ));
            }
        } else {
            return Err(miette!("subprocess_storage_name not in registry"));
        }

        if !asset_storage_registry_lock.contains_key(output_storage_name) {
            return Err(miette!("output_storage_name not in registry"));
        }

        let background_asset_storage_registry = Arc::clone(asset_storage_registry);
        let (task_sender, task_receiver) = mpsc::channel(64);
        let (event_sender, event_receiver) = mpsc::channel(64);
        let (cancel_sender, cancel_receiver) = mpsc::channel(64);
        tokio::spawn(process_tasks(
            task_receiver,
            cancel_receiver,
            event_sender,
            background_asset_storage_registry,
        ));

        let asset_storage_registry = Arc::clone(asset_storage_registry);
        Ok(Self {
            id_source: AtomicU32::new(0),

            task_sender,
            cancel_sender,
            event_receiver: Mutex::new(Some(event_receiver)),
            subprocess_storage_name: subprocess_storage_name.to_string(),
            output_storage_name: output_storage_name.to_string(),
            asset_storage_registry,
        })
    }

    async fn workers(&self) -> miette::Result<Vec<WorkerSpec>> {
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

    fn build_inputs(
        &self,
        inputs: &HashMap<String, AssetSpec>,
        base: &std::path::Path,
    ) -> Result<HashMap<String, PathBuf>, miette::Error> {
        let inputs = transfer_assets(
            &self.asset_storage_registry,
            &self.subprocess_storage_name,
            inputs,
        )?;
        let inputs =
            write_input_paths(&inputs).wrap_err("Failed to collect Worker input filepaths")?.iter().map(|(k, v)| {
                let rel_path = v.strip_prefix(base).into_diagnostic()?;
                Ok((k.clone(), rel_path.to_path_buf()))
            }).collect::<Result<HashMap<_, _>, miette::Error>>()?;
        Ok(inputs)
    }

    fn build_outputs(&self, outputs: HashSet<String>, base: &std::path::Path) -> Result<OutputSpecs, miette::Error> {
        let output_specs = reserve_asset_specs(
            &self.asset_storage_registry,
            &self.subprocess_storage_name,
            outputs.len(),
        )?;
        let outputs: HashMap<String, AssetSpec> = outputs.into_iter().zip(output_specs).collect();
        let output_paths = outputs
            .iter()
            .map(|(k, v)| {
                let path = v.path()?;
                let rel_path = path.strip_prefix(base).into_diagnostic()?;
                Ok((k.clone(), rel_path.to_path_buf()))
            })
            .collect::<Result<HashMap<_, _>, miette::Error>>()?;
        Ok((outputs, output_paths))
    }
}

fn read_stderr(child: &mut tokio::process::Child) -> tokio::task::JoinHandle<String> {
    let mut stderr = child.stderr.take().unwrap();
    tokio::spawn(async move {
        let mut stderr_out = String::new();
        let _ = stderr.read_to_string(&mut stderr_out).await;

        stderr_out
    })
}

// Json format expected for subprocess workers.
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

// Write the paths of the inputs to the worker.
fn write_input_paths(
    inputs: &HashMap<String, AssetSpec>,
) -> miette::Result<HashMap<String, PathBuf>> {
    let inputs_len = inputs.len();
    let mut input_paths = HashMap::with_capacity(inputs_len);
    for (key, value) in inputs {
        let path = value.path()?;
        input_paths.insert(key.clone(), path);
    }

    Ok(input_paths)
}

impl Executor for HPCExecutor {
    fn workers(&self) -> BoxFuture<'_, miette::Result<Vec<WorkerSpec>>> {
        self.workers().boxed()
    }

    fn execute(&self, task_plans: Vec<TaskPlan>) -> BoxFuture<'_, miette::Result<Vec<u32>>> {
        let fut = async {
            let mut ids = Vec::new();
            let mut task_sender = self.task_sender.clone();

            let base_path = self.asset_storage_registry
                .read()
                .unwrap()
                .get(&self.subprocess_storage_name)
                .ok_or_else(|| miette!("subprocess_storage_name not in registry"))
                .and_then(|file_storage| match file_storage.kind() {
                    AssetKind::File { root } => Ok(root),
                    _ => Err(miette!("subprocess_storage_name must be of AssetKind::File")),
                })?;

            for task_plan in task_plans {
                let id = self
                    .id_source
                    .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                let inputs = self.build_inputs(&task_plan.inputs, &base_path.as_path())?;
                let (outputs, output_paths) = self.build_outputs(task_plan.outputs, &base_path.as_path())?;

                
                let tmp_assets = &reserve_asset_specs(&self.asset_storage_registry, &self.subprocess_storage_name, 2)?;
                let worker_args = tmp_assets[0].path()?;
                let file = std::fs::File::create(&worker_args).into_diagnostic()?;
                let script_path = tmp_assets[1].path()?;
                // Redirect the done_file to a temporary file as we
                // do not need it to figure out if a process has
                // completed currently.
                let done_file = std::path::Path::new( "_done").to_path_buf();

                serde_json::to_writer(
                    file,
                    &WorkerCallArgs {
                        function_name: task_plan.task_name,
                        inputs,
                        outputs: output_paths,
                        done_path: done_file.clone(),
                        ..Default::default()
                    },
                )
                .into_diagnostic()?;

                let output_storage_name = task_plan
                    .output_storage_name
                    .clone()
                    .unwrap_or_else(|| self.output_storage_name.clone());

                task_sender
                    .send(BackgroundTaskPlan {
                        id,
                        worker_name: task_plan.worker_name,
                        output_storage_name,
                        worker_args,
                        script_path,
                        done_file,
                        outputs,
                        resources: HPCResourceSpec::default(),
                        environment: HPCEnvironmentSpec::default(),
                    })
                    .await
                    .into_diagnostic()?;

                ids.push(id);
            }

            Ok(ids)
        };
        fut.boxed()
    }

    fn listen(&self) -> miette::Result<BoxStream<'_, Event>> {
        // Explicit block to allow us to drop the MutexGuard after we
        // take the receiver.
        let channel = {
            let mut receiver = self
                .event_receiver
                .try_lock()
                .map_err(|err| miette!("Failed to listen: {}", err))?;

            receiver.take().ok_or(miette!(
                "Failed to listen: Executor is already being listened to."
            ))?
        };
        Ok(channel.boxed())
    }

    fn cancel(&self, task_ids: Vec<u32>) -> BoxFuture<'_, miette::Result<()>> {
        let fut = async {
            let mut cancel_sender = self.cancel_sender.clone();
            for task_id in task_ids {
                cancel_sender.send(task_id).await.into_diagnostic()?;
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

    use crate::{asset_storage::{load_checkpoints_dir,assert_registry_contains_values,  FileAssetStorage, test_storage_registry}, executor::HPCExecutor};
    use super::*;
    // Test that we can launch a task and listen for
    // errors when they occur
    #[tokio::test]
    async fn execute_subprocess_error() -> miette::Result<()> {
        // TODO: overwrite test_storage_registry in a way that the file system is the checkpoints dir
        let file_storage = FileAssetStorage::new(std::path::Path::new("/Users/philipp.seitz/.tierkreis/checkpoints/00000000-0000-0000-0000-000000000016/"));
        let (registry, input_sets, _dir) = test_storage_registry(vec![json!({"value": "Test"})], vec![]);
        registry.write().unwrap().insert("checkpoints".to_string(), Box::new(file_storage));
        let mut outputs = HashSet::new();
        outputs.insert("value".to_string());
        let task_plans = vec![TaskPlan {
            worker_name: "hello-world-worker".to_string(),
            task_name: "mpi_rank_info_with_input".to_string(),
            outputs,
            inputs: input_sets[0].clone(),

            ..Default::default()
        }];
        let executor = HPCExecutor::try_new(&registry, "checkpoints", "checkpoints")?;

        let stream = executor.listen()?;
        executor.execute(task_plans).await?;

        let events = stream.take(2).collect::<Vec<_>>().await;
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].status, Status::Running);
        assert_registry_contains_values(
            &registry,
            "checkpoints",
            &events[1].clone().outputs().unwrap_or_default(),
            json!({"value": "Rank 0 out of 2 on c1 with value Test.\nRank 1 out of 2 on c2 with value Test."}),
        );

        Ok(())
    }
    
}
