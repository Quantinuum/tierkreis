/*!
This module defines the [`SubprocessExecutor`] struct which implements [Executor]
by running subprocesses.
*/
use std::{
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
    process::{ExitStatus, Stdio},
    sync::{Arc, Mutex, atomic::AtomicU32},
    any::Any,
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
    process::Command,
    task::{AbortHandle, JoinHandle},
};
use which::which_re;

use crate::{
    asset_storage::{
        AssetKind, AssetSpec, AssetStorageRegistry, reserve_asset_specs, transfer_assets,
    },
    event::{Event, Status, send_cancelled, send_complete, send_error, send_running},
    executor::interface::{Executor, TaskPlan, WorkerSpec},
};

/// [`SubprocessResourceSpec`] determines what Resources should be available to the
/// [`SubprocessExecutor`] or what is requested as part of a [`TaskPlan`].
#[derive(Clone, Debug, PartialEq, Default)]
pub struct SubprocessResourceSpec {}

/// [`SubprocessEnvironmentSpec`] determines the default execution environment of
/// [`SubprocessExecutor`] or what is requested as part of a [`TaskPlan`].
#[derive(Clone, Debug, PartialEq, Default)]
pub struct SubprocessEnvironmentSpec {}

struct BackgroundTaskPlan {
    id: u32,
    worker_name: String,
    output_storage_name: String,
    worker_args: NamedTempFile,
    done_file: NamedTempFile,
    outputs: HashMap<String, AssetSpec>,
}

struct BackgroundTask {
    id: u32,
    output_storage_name: String,
    exit_status: Result<ExitStatus, std::io::Error>,
    outputs: HashMap<String, AssetSpec>,
    stderr: JoinHandle<String>,

    // Handles to temporary files to prevent deletion
    // until the task is complete.
    _worker_args: NamedTempFile,
    _done_file: NamedTempFile,
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

    let worker_args = internal_task.worker_args;
    let worker_args_path = worker_args.path();
    let done_file = internal_task.done_file;
    let res = spawn_worker(&internal_task.worker_name, worker_args_path);
    let mut child = match res {
        Ok(child) => child,
        Err(err) => {
            send_error(event_sender, id, &err).await?;
            return Ok(());
        }
    };
    let stderr = read_stderr(&mut child);

    let outputs = internal_task.outputs;
    let output_storage_name = internal_task.output_storage_name;
    let task = tokio::task::spawn(async move {
        let exit_status = child.wait().await;
        BackgroundTask {
            id,
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
pub struct SubprocessExecutor {
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

impl SubprocessExecutor {
    /// Try to create a new [`SubprocessExecutor`] with an [`AssetStorageRegistry`], a
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
    ) -> Result<HashMap<String, PathBuf>, miette::Error> {
        let inputs = transfer_assets(
            &self.asset_storage_registry,
            &self.subprocess_storage_name,
            inputs,
        )?;
        let inputs =
            write_input_paths(&inputs).wrap_err("Failed to collect Worker input filepaths")?;
        Ok(inputs)
    }

    fn build_outputs(&self, outputs: HashSet<String>) -> Result<OutputSpecs, miette::Error> {
        let output_specs = reserve_asset_specs(
            &self.asset_storage_registry,
            &self.subprocess_storage_name,
            outputs.len(),
        )?;
        let outputs: HashMap<String, AssetSpec> = outputs.into_iter().zip(output_specs).collect();
        let output_paths = outputs
            .iter()
            .map(|(k, v)| Ok((k.clone(), v.path()?)))
            .collect::<Result<HashMap<_, _>, miette::Error>>()?;
        Ok((outputs, output_paths))
    }
}

fn spawn_worker(
    worker_name: &str,
    worker_args_path: &Path,
) -> Result<tokio::process::Child, miette::Error> {
    let child = Command::new(format!("tkr-{worker_name}"))
        .arg(worker_args_path)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .into_diagnostic()
        .wrap_err(miette!("Could not spawn worker"))?;
    Ok(child)
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

impl Executor for SubprocessExecutor {
    fn workers(&self) -> BoxFuture<'_, miette::Result<Vec<WorkerSpec>>> {
        self.workers().boxed()
    }
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn execute(&self, task_plans: Vec<TaskPlan>) -> BoxFuture<'_, miette::Result<Vec<u32>>> {
        let fut = async {
            let mut ids = Vec::new();
            let mut task_sender = self.task_sender.clone();

            for task_plan in task_plans {
                let id = self
                    .id_source
                    .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                let inputs = self.build_inputs(&task_plan.inputs)?;
                let (outputs, output_paths) = self.build_outputs(task_plan.outputs)?;

                let worker_args = NamedTempFile::new().into_diagnostic()?;

                // Redirect the done_file to a temporary file as we
                // do not need it to figure out if a process has
                // completed currently.
                let done_file = NamedTempFile::new().into_diagnostic()?;

                serde_json::to_writer(
                    &worker_args,
                    &WorkerCallArgs {
                        function_name: task_plan.task_name,
                        inputs,
                        outputs: output_paths,
                        done_path: done_file.path().to_path_buf(),
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
                        done_file,
                        outputs,
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
    use std::collections::HashSet;

    use futures::StreamExt;
    use rstest::rstest;
    use serde_json::json;

    use crate::asset_storage::{assert_registry_contains_values, test_storage_registry};

    use super::*;

    // Test that we can list the available workers in $PATH
    #[tokio::test]
    async fn subprocess_workers() -> miette::Result<()> {
        let (registry, _, _) = test_storage_registry(vec![], vec![]);
        let executor = SubprocessExecutor::try_new(&registry, "file", "file")?;

        let workers = executor.workers().await?;

        // We should expect these workers to be already installed.
        assert!(
            workers
                .iter()
                .any(|workers| workers.worker_name == "tkr-aer-worker")
        );
        assert!(
            workers
                .iter()
                .any(|workers| workers.worker_name == "tkr-qulacs-worker")
        );

        // We should not include the main cli tool in the worker list.
        assert!(workers.iter().all(|workers| workers.worker_name != "tkr"));

        Ok(())
    }

    // Test that we can launch a single task and listen for
    // the status changes.
    #[rstest]
    #[case("memory")]
    #[case("file")]
    #[tokio::test]
    async fn execute_subprocess(#[case] output_storage_name: &str) -> miette::Result<()> {
        let (registry, input_sets, _dir) = test_storage_registry(
            vec![json!({"greeting": "hello ", "subject": "dave"})],
            vec![],
        );
        let mut outputs = HashSet::new();
        outputs.insert("value".to_string());

        let task_plans = vec![TaskPlan {
            worker_name: "hello-world-worker".to_string(),
            task_name: "greet".to_string(),

            inputs: input_sets[0].clone(),
            outputs,
            ..Default::default()
        }];
        let executor = SubprocessExecutor::try_new(&registry, "file", output_storage_name)?;

        let stream = executor.listen()?;
        executor.execute(task_plans).await?;

        let events = stream.take(2).collect::<Vec<_>>().await;
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].status, Status::Running);
        assert_registry_contains_values(
            &registry,
            output_storage_name,
            &events[1].clone().outputs().unwrap_or_default(),
            json!({"value": "hello dave"}),
        );

        Ok(())
    }

    // Test that we can launch a single task and listen for
    // the status changes when reading the input values from
    // a FileAssetStorage.
    #[rstest]
    #[case("memory")]
    #[case("file")]
    #[tokio::test]
    async fn execute_subprocess_with_file_inputs(
        #[case] output_storage_name: &str,
    ) -> miette::Result<()> {
        let (registry, input_sets, _dir) = test_storage_registry(
            vec![],
            vec![json!({"greeting": "hello ", "subject": "dave"})],
        );
        let mut outputs = HashSet::new();
        outputs.insert("value".to_string());

        let task_plans = vec![TaskPlan {
            worker_name: "hello-world-worker".to_string(),
            task_name: "greet".to_string(),

            inputs: input_sets[0].clone(),
            outputs,

            ..Default::default()
        }];
        let executor = SubprocessExecutor::try_new(&registry, "file", output_storage_name)?;

        let stream = executor.listen()?;
        executor.execute(task_plans).await?;

        let events = stream.take(2).collect::<Vec<_>>().await;
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].status, Status::Running);
        assert_registry_contains_values(
            &registry,
            output_storage_name,
            &events[1].clone().outputs().unwrap_or_default(),
            json!({"value": "hello dave"}),
        );

        Ok(())
    }

    // Test that we can launch two tasks and listen for
    // their status changes.
    #[rstest]
    #[case("memory")]
    #[case("file")]
    #[tokio::test]
    async fn execute_subprocess_two_tasks(#[case] output_storage_name: &str) -> miette::Result<()> {
        let (registry, input_sets, _dir) = test_storage_registry(
            vec![
                json!({"greeting": "hello ", "subject": "dave"}),
                json!({"greeting": "hi ", "subject": "steve"}),
            ],
            vec![],
        );
        let mut outputs = HashSet::new();
        outputs.insert("value".to_string());

        let task_plans = vec![
            TaskPlan {
                worker_name: "hello-world-worker".to_string(),
                task_name: "greet".to_string(),

                inputs: input_sets[0].clone(),
                outputs: outputs.clone(),

                ..Default::default()
            },
            TaskPlan {
                worker_name: "hello-world-worker".to_string(),
                task_name: "greet".to_string(),

                inputs: input_sets[1].clone(),
                outputs: outputs.clone(),

                ..Default::default()
            },
        ];
        let executor = SubprocessExecutor::try_new(&registry, "file", output_storage_name)?;

        let stream = executor.listen()?;
        executor.execute(task_plans).await?;

        let events = stream.take(4).collect::<Vec<_>>().await;
        assert_eq!(events.len(), 4);
        assert!(events.contains(&Event {
            id: 0,
            status: Status::Running
        }));
        assert!(events.contains(&Event {
            id: 1,
            status: Status::Running
        }));

        // These may complete out of order, so find the correct events.
        let complete0 = events
            .iter()
            .find(|event| event.is_complete() && event.id == 0)
            .unwrap();
        assert_registry_contains_values(
            &registry,
            output_storage_name,
            &complete0.clone().outputs().unwrap_or_default(),
            json!({"value": "hello dave"}),
        );
        let complete1 = events
            .iter()
            .find(|event| event.is_complete() && event.id == 1)
            .unwrap();
        assert_registry_contains_values(
            &registry,
            output_storage_name,
            &complete1.clone().outputs().unwrap_or_default(),
            json!({"value": "hi steve"}),
        );

        Ok(())
    }

    // Test that we can launch a task and listen for
    // the status changes even if we launch the task before
    // we start listening to the executor.
    #[tokio::test]
    async fn execute_subprocess_execute_before_listen() -> miette::Result<()> {
        let (registry, input_sets, _dir) = test_storage_registry(
            vec![json!({"greeting": "hello ", "subject": "dave"})],
            vec![],
        );
        let mut outputs = HashSet::new();
        outputs.insert("value".to_string());

        let task_plans = vec![TaskPlan {
            worker_name: "hello-world-worker".to_string(),
            task_name: "greet".to_string(),

            inputs: input_sets[0].clone(),
            outputs: outputs.clone(),

            ..Default::default()
        }];
        let executor = SubprocessExecutor::try_new(&registry, "file", "file")?;

        executor.execute(task_plans).await?;
        let stream = executor.listen()?;

        let events = stream.take(2).collect::<Vec<_>>().await;
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].status, Status::Running);
        assert_registry_contains_values(
            &registry,
            "file",
            &events[1].clone().outputs().unwrap_or_default(),
            json!({"value": "hello dave"}),
        );

        Ok(())
    }

    // Test that we can launch a task and listen for
    // errors when they occur
    #[tokio::test]
    async fn execute_subprocess_error() -> miette::Result<()> {
        let (registry, _, _dir) = test_storage_registry(vec![], vec![]);
        let task_plans = vec![TaskPlan {
            worker_name: "hello-world-worker".to_string(),
            // hello-world-worker has no task called "hail"
            task_name: "hail".to_string(),

            inputs: HashMap::new(),

            ..Default::default()
        }];
        let executor = SubprocessExecutor::try_new(&registry, "file", "file")?;

        let stream = executor.listen()?;
        executor.execute(task_plans).await?;

        let events = stream.take(2).collect::<Vec<_>>().await;
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].status, Status::Running);
        assert_eq!(
            events[1].status,
            Status::Error {
                error: "Could not spawn worker".to_string(),
                detail: None,
            }
        );
        Ok(())
    }

    // Test that we can launch a task and then cancel it
    // before it completes.
    #[tokio::test]
    async fn execute_subprocess_cancel() -> miette::Result<()> {
        let (registry, input_sets, _dir) = test_storage_registry(
            vec![json!({"greeting": "hello ", "subject": "dave"})],
            vec![],
        );
        let mut outputs = HashSet::new();
        outputs.insert("value".to_string());

        let task_plans = vec![TaskPlan {
            worker_name: "hello-world-worker".to_string(),
            task_name: "greet".to_string(),

            inputs: input_sets[0].clone(),
            outputs: outputs.clone(),

            ..Default::default()
        }];
        let executor = SubprocessExecutor::try_new(&registry, "file", "file")?;

        let mut stream = executor.listen()?;
        let task_ids = executor.execute(task_plans).await?;

        let event = stream.next().await.unwrap();
        assert_eq!(event.status, Status::Running);

        executor.cancel(task_ids).await?;

        let event = stream.next().await.unwrap();
        assert_eq!(event.status, Status::Cancelled);

        Ok(())
    }

    // Test that we can pass a non-existent ID to cancel and
    // it will not error.
    #[tokio::test]
    async fn execute_subprocess_cancel_non_existent() -> miette::Result<()> {
        let (registry, _, _) = test_storage_registry(vec![], vec![]);
        let executor = SubprocessExecutor::try_new(&registry, "file", "file")?;

        executor.cancel(vec![0]).await?;

        Ok(())
    }

    // Test that we can pass a completed ID to cancel and
    // it will not error.
    #[tokio::test]
    async fn execute_subprocess_cancel_completed() -> miette::Result<()> {
        let (registry, input_sets, _dir) = test_storage_registry(
            vec![json!({"greeting": "hello ", "subject": "dave"})],
            vec![],
        );
        let mut outputs = HashSet::new();
        outputs.insert("value".to_string());

        let task_plans = vec![TaskPlan {
            worker_name: "hello-world-worker".to_string(),
            task_name: "greet".to_string(),

            inputs: input_sets[0].clone(),
            outputs: outputs.clone(),

            ..Default::default()
        }];
        let executor = SubprocessExecutor::try_new(&registry, "file", "file")?;

        let stream = executor.listen()?;
        let task_ids = executor.execute(task_plans).await?;

        let events = stream.take(2).collect::<Vec<_>>().await;
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].status, Status::Running);
        assert_registry_contains_values(
            &registry,
            "file",
            &events[1].clone().outputs().unwrap_or_default(),
            json!({"value": "hello dave"}),
        );

        executor.cancel(task_ids).await?;

        Ok(())
    }
}
