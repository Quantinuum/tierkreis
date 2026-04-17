use std::{
    collections::HashMap,
    path::PathBuf,
    process::Stdio,
    sync::{Arc, Mutex},
};

use futures::{
    FutureExt, StreamExt,
    channel::{mpsc, oneshot},
    future::BoxFuture,
    stream::{self, BoxStream},
};
use miette::{Context, IntoDiagnostic, miette};
use regex::Regex;
use serde::{Deserialize, Serialize};
use tempfile::NamedTempFile;
use tokio::{io::AsyncReadExt, process::Command};
use which::which_re;

use crate::{
    asset_storage::{
        AssetStorageRegistry, interface::AssetSpec, reserve_asset_specs, transfer_assets,
    },
    executor::interface::{Event, Executor, Status, TaskPlan, WorkerSpec},
};

#[derive(Clone, Debug, PartialEq, Default)]
pub struct SubprocessResourceSpec {}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct SubprocessEnvironmentSpec {}

pub struct SubprocessExecutor {
    event_sender: mpsc::Sender<Event<u32>>,
    event_receiver: Mutex<Option<mpsc::Receiver<Event<u32>>>>,
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

impl SubprocessExecutor {
    pub fn try_new(
        asset_storage_registry: &AssetStorageRegistry,
        subprocess_storage_name: &str,
        output_storage_name: &str,
    ) -> miette::Result<Self> {
        let asset_storage_registry_lock = asset_storage_registry
            .read()
            .map_err(|err| miette!("Failed to lock AssetStorageRegistry: {err}"))?;
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
    }

    async fn execute(&self, task_plans: Vec<TaskPlan>) -> miette::Result<Vec<u32>> {
        let mut pids = Vec::new();
        let mut tasks = Vec::new();

        for task_plan in task_plans.into_iter() {
            let inputs = transfer_assets(
                &self.asset_storage_registry,
                &self.subprocess_storage_name,
                task_plan.inputs,
            )?;
            let inputs =
                write_input_paths(inputs).wrap_err("Failed to collect Worker input filepaths")?;
            let output_specs = reserve_asset_specs(
                &self.asset_storage_registry,
                &self.subprocess_storage_name,
                task_plan.outputs.len(),
            )?;
            let outputs: HashMap<String, AssetSpec> =
                task_plan.outputs.into_iter().zip(output_specs).collect();
            let output_paths = outputs
                .iter()
                .map(|(k, v)| Ok((k.clone(), v.path()?)))
                .collect::<Result<HashMap<_, _>, miette::Error>>()?;

            let worker_args = NamedTempFile::new().into_diagnostic()?;
            let worker_args_path = worker_args
                .path()
                .canonicalize()
                .into_diagnostic()?
                .into_os_string();

            serde_json::to_writer(
                &worker_args,
                &WorkerCallArgs {
                    function_name: task_plan.task_name,
                    inputs,
                    outputs: output_paths,
                    ..Default::default()
                },
            )
            .into_diagnostic()?;

            let mut child = Command::new(format!("tkr-{}", task_plan.worker_name))
                .arg(worker_args_path)
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
                .into_diagnostic()
                .wrap_err(miette!("Could not spawn worker"))?;

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
                        detail: None,
                    })
                    .expect("Failed to send Running event.");

                let mut stdout = child.stdout.take().unwrap();
                let read_stdout = tokio::spawn(async move {
                    let mut stdout_out = String::new();
                    let _ = stdout.read_to_string(&mut stdout_out).await;

                    stdout_out
                });

                let mut stderr = child.stderr.take().unwrap();
                let read_stderr = tokio::spawn(async move {
                    let mut stderr_out = String::new();
                    let _ = stderr.read_to_string(&mut stderr_out).await;

                    stderr_out
                });

                tokio::select! {
                    Ok(()) = cancel_receiver => {
                        child.kill().await.expect("Failed to kill child process.");
                        event_sender
                            .try_send(Event {
                                id,
                                status: Status::Cancelled,
                                detail: None,
                            })
                            .expect("Failed to send Cancelled event.")
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
                                            detail: None,
                                        })
                                        .expect("Failed to send Complete event.")
                                } else {
                                    let stdout = read_stdout.await.ok();
                                    let _stderr = read_stderr.await.ok();
                                    event_sender
                                        .try_send(Event {
                                            id,
                                            status: Status::Error {error: format!("Subprocess failed with exit code: {}", status) },
                                            detail: stdout,
                                        })
                                        .expect("Failed to send Error event.");
                                }
                            }
                            Err(err) => {
                                event_sender
                                    .try_send(Event {
                                        id,
                                        status: Status::Error {error: err.to_string()},
                                        detail: None,
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
        input_paths.insert(key.to_string(), path);
    }

    Ok(input_paths)
}

impl Executor for SubprocessExecutor {
    fn workers(&self) -> BoxFuture<'_, miette::Result<Vec<WorkerSpec>>> {
        self.workers().boxed()
    }

    fn execute(&self, task_plans: Vec<TaskPlan>) -> BoxFuture<'_, miette::Result<Vec<u32>>> {
        self.execute(task_plans).boxed()
    }

    fn listen(&self) -> miette::Result<BoxStream<'_, Event<u32>>> {
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
        for task_id in task_ids.into_iter() {
            match cancel_senders.remove(&task_id) {
                Some(cancel_sender) => {
                    // We can ignore send errors as they mean the other side
                    // of the channel has closed.
                    let _res = cancel_sender.send(());
                }
                None => continue,
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
        assert_eq!(
            events[1].status,
            Status::Error {
                error: "Subprocess failed with exit code: exit status: 1".to_string()
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

        let stream = executor.listen()?;
        let task_ids = executor.execute(task_plans).await?;
        executor.cancel(task_ids)?;

        let events = stream.take(2).collect::<Vec<_>>().await;
        dbg!(&events);
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].status, Status::Running);
        assert_eq!(events[1].status, Status::Cancelled);

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
