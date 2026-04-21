/*!
This module defines the [`SubprocessExecutor`] struct which implements [Executor]
by running subprocesses.
*/
use std::{
    collections::{HashMap, HashSet},
    path::PathBuf,
    process::Stdio,
    sync::{Arc, Mutex},
};

use futures::{
    FutureExt, StreamExt,
    channel::{mpsc, oneshot},
    future::{self, BoxFuture},
    stream::{self, BoxStream},
};
use miette::{Context, IntoDiagnostic, miette};
use regex::Regex;
use serde::{Deserialize, Serialize};
use tempfile::NamedTempFile;
use tokio::{io::AsyncReadExt, process::Command};
use which::which_re;

use crate::{
    asset_storage::{AssetSpec, AssetStorageRegistry, reserve_asset_specs, transfer_assets},
    event::{Event, Status},
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

/// [`SubprocessExecutor`] defines an [Executor] that performs Task Nodes using Worker subprocesses.
pub struct SubprocessExecutor {
    event_sender: mpsc::Sender<Event>,
    event_receiver: Mutex<Option<mpsc::Receiver<Event>>>,
    cancel_senders: Mutex<HashMap<u32, oneshot::Sender<()>>>,

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
        if !asset_storage_registry_lock.contains_key(subprocess_storage_name) {
            return Err(miette!("subprocess_storage_name not in registry"));
        }

        if !asset_storage_registry_lock.contains_key(output_storage_name) {
            return Err(miette!("output_storage_name not in registry"));
        }

        let (sender, receiver) = mpsc::channel(128);
        let asset_storage_registry = Arc::clone(asset_storage_registry);
        Ok(Self {
            event_sender: sender,
            event_receiver: Mutex::new(Some(receiver)),
            cancel_senders: Mutex::new(HashMap::new()),
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

    fn internal_execute(&self, task_plans: Vec<TaskPlan>) -> miette::Result<Vec<u32>> {
        let mut pids = Vec::new();
        let mut tasks = Vec::new();

        for task_plan in task_plans {
            let inputs = self.build_inputs(task_plan.inputs)?;
            let (outputs, output_paths) = self.build_outputs(task_plan.outputs)?;

            let worker_args = NamedTempFile::new().into_diagnostic()?;
            let worker_args_path = worker_args
                .path()
                .canonicalize()
                .into_diagnostic()?
                .into_os_string();

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

            let mut child = spawn_worker(&task_plan.worker_name, worker_args_path)?;

            let id = child.id().ok_or(miette!("Could not get process id."))?;
            pids.push(id);

            let (cancel_sender, cancel_receiver) = oneshot::channel();
            let mut cancel_senders = self.cancel_senders.lock().unwrap();
            cancel_senders.insert(id, cancel_sender);

            let mut event_sender = self.event_sender.clone();
            let asset_storage_registry = self.asset_storage_registry.clone();
            let output_storage_name = task_plan
                .output_storage_name
                .unwrap_or_else(|| self.output_storage_name.clone());
            let task = async move {
                // Move the temporary file into this task so it will not be deleted
                // until after the process has started properly.
                let _worker_args = worker_args;
                event_sender
                    .try_send(Event {
                        id,
                        status: Status::Running,
                    })
                    .expect("Failed to send Running event.");

                let read_stdout = read_stdout(&mut child);

                let read_stderr = read_stderr(&mut child);

                tokio::select! {
                    Ok(()) = cancel_receiver => {
                        child.kill().await.expect("Failed to kill child process.");
                        event_sender
                            .try_send(Event {
                                id,
                                status: Status::Cancelled,
                            })
                            .expect("Failed to send Cancelled event.");
                    }
                    res = child.wait() => {
                        match res {
                            Ok(status) => {
                                if status.success() {
                                    let outputs = transfer_assets(
                                        &asset_storage_registry,
                                        &output_storage_name,
                                        outputs,
                                    ).unwrap();
                                    event_sender
                                        .try_send(Event {
                                            id,
                                            status: Status::Complete { outputs },
                                        })
                                        .expect("Failed to send Complete event.");
                                } else {
                                    let _stdout = read_stdout.await.ok();
                                    let stderr = read_stderr.await.ok();
                                    event_sender
                                        .try_send(Event {
                                            id,
                                            status: Status::Error {error: format!("Subprocess failed with exit code: {status}"), detail: stderr },
                                        })
                                        .expect("Failed to send Error event.");
                                }
                            }
                            Err(err) => {
                                event_sender
                                    .try_send(Event {
                                        id,
                                        status: Status::Error {error: err.to_string(), detail: None},
                                    })
                                    .expect("Failed to send Error event.");
                            }
                        }
                    }
                }
            };

            tasks.push(task);
        }

        // This guarantees the tasks will complete in order as far as the runtime
        // is concerned.
        tokio::spawn(stream::iter(tasks).fold((), |(), x| async move {
            x.await;
        }));

        Ok(pids)
    }

    fn build_inputs(
        &self,
        inputs: HashMap<String, AssetSpec>,
    ) -> Result<HashMap<String, PathBuf>, miette::Error> {
        let inputs = transfer_assets(
            &self.asset_storage_registry,
            &self.subprocess_storage_name,
            inputs,
        )?;
        let inputs =
            write_input_paths(inputs).wrap_err("Failed to collect Worker input filepaths")?;
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
    worker_args_path: std::ffi::OsString,
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

fn read_stdout(child: &mut tokio::process::Child) -> tokio::task::JoinHandle<String> {
    let mut stdout = child.stdout.take().unwrap();
    tokio::spawn(async move {
        let mut stdout_out = String::new();
        let _ = stdout.read_to_string(&mut stdout_out).await;

        stdout_out
    })
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
    inputs: HashMap<String, AssetSpec>,
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

    fn execute(&self, task_plans: Vec<TaskPlan>) -> BoxFuture<'_, miette::Result<Vec<u32>>> {
        future::ready(self.internal_execute(task_plans)).boxed()
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

    fn cancel(&self, task_ids: Vec<u32>) -> miette::Result<()> {
        let mut cancel_senders = self
            .cancel_senders
            .lock()
            .map_err(|err| miette!("Failed to lock cancel channels: {}", err))?;
        for task_id in task_ids {
            if let Some(cancel_sender) = cancel_senders.remove(&task_id) {
                // We can ignore send errors as they mean the other side
                // of the channel has closed.
                let _res = cancel_sender.send(());
            }
        }
        Ok(())
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
        dbg!(&events);
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
        dbg!(&events);
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
        dbg!(&events);
        assert_eq!(events.len(), 4);
        assert_eq!(events[0].status, Status::Running);
        assert_registry_contains_values(
            &registry,
            output_storage_name,
            &events[1].clone().outputs().unwrap_or_default(),
            json!({"value": "hello dave"}),
        );
        assert_eq!(events[2].status, Status::Running);
        assert_registry_contains_values(
            &registry,
            output_storage_name,
            &events[3].clone().outputs().unwrap_or_default(),
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
        dbg!(&events);
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
        dbg!(&events);
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].status, Status::Running);
        matches!(
            &events[1].status,
            Status::Error {
                error,
                ..
            } if error == "Subprocess failed with exit code: exit status: 1",
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

        executor.cancel(task_ids)?;

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

        executor.cancel(vec![0])?;

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
        dbg!(&events);
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].status, Status::Running);
        assert_registry_contains_values(
            &registry,
            "file",
            &events[1].clone().outputs().unwrap_or_default(),
            json!({"value": "hello dave"}),
        );

        executor.cancel(task_ids)?;

        Ok(())
    }
}
