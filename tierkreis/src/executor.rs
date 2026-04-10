use std::{collections::HashMap, path::PathBuf, sync::Mutex};

use futures::{Stream, stream};
use miette::{IntoDiagnostic, miette};
use regex::Regex;
use serde::{Deserialize, Serialize};
use tempfile::NamedTempFile;
use tokio::{
    process::Command,
    sync::{mpsc, oneshot},
};
use which::which_re;

#[derive(Debug, PartialEq)]
pub struct Event<ID> {
    id: ID,
    status: Status,
}

#[derive(Debug, PartialEq)]
pub enum Status {
    Queued,
    Running,
    Complete,
    Cancelled,
    Error,
}

pub struct TaskPlan<ResourceSpec, EnvironmentSpec> {
    pub worker_name: String,
    pub task_name: String,
    pub inputs: HashMap<String, String>,
    pub outputs: HashMap<String, String>,

    pub resources: ResourceSpec,
    pub environment: EnvironmentSpec,
}

#[derive(Debug, PartialEq)]
pub struct WorkerSpec {
    pub worker_name: String,
}

pub trait Executor {
    type ID;
    type ResourceSpec;
    type EnvironmentSpec;

    fn workers(&self) -> impl Future<Output = miette::Result<Vec<WorkerSpec>>>;

    fn execute(
        &self,
        task_plans: impl IntoIterator<Item = TaskPlan<Self::ResourceSpec, Self::EnvironmentSpec>>,
    ) -> impl Future<Output = miette::Result<Vec<Self::ID>>>;

    fn listen(&self) -> miette::Result<impl Stream<Item = Event<Self::ID>>>;

    fn reconnect(&self, task_ids: impl IntoIterator<Item = Self::ID>);

    fn cancel(&self, task_ids: impl IntoIterator<Item = Self::ID>);

    fn restart(&self, task_ids: impl IntoIterator<Item = Self::ID>);
}

struct SubprocessResourceSpec {}

struct SubprocessEnvironmentSpec {}

struct SubprocessExecutor {
    event_sender: mpsc::Sender<Event<u32>>,
    event_receiver: Mutex<Option<mpsc::Receiver<Event<u32>>>>,
    cancel_senders: Mutex<HashMap<u32, oneshot::Sender<()>>>,
}

impl SubprocessExecutor {
    pub fn new() -> Self {
        let (sender, receiver) = mpsc::channel(128);
        Self {
            event_sender: sender,
            event_receiver: Mutex::new(Some(receiver)),
            cancel_senders: Mutex::new(HashMap::new()),
        }
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

impl Executor for SubprocessExecutor {
    type ID = u32; // pid
    type ResourceSpec = SubprocessResourceSpec;
    type EnvironmentSpec = SubprocessEnvironmentSpec;

    async fn workers(&self) -> miette::Result<Vec<WorkerSpec>> {
        let re = Regex::new(r"tkr-.*-worker").unwrap();
        let paths = which_re(&re).into_diagnostic()?;
        Ok(paths
            .map(|path| WorkerSpec {
                worker_name: path.file_name().unwrap().to_str().unwrap().to_string(),
            })
            .collect())
    }

    async fn execute(
        &self,
        task_plans: impl IntoIterator<Item = TaskPlan<Self::ResourceSpec, Self::EnvironmentSpec>>,
    ) -> miette::Result<Vec<Self::ID>> {
        let mut pids = Vec::new();

        for task_plan in task_plans.into_iter() {
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
                    ..Default::default()
                },
            )
            .into_diagnostic()?;

            let mut child = Command::new(format!("tkr-{}", task_plan.worker_name))
                .arg(worker_args_path)
                .spawn()
                .into_diagnostic()?;

            let id = child.id().ok_or(miette!("Could not get process id."))?;
            pids.push(id);

            let (cancel_sender, cancel_receiver) = oneshot::channel();
            let mut cancel_senders = self.cancel_senders.lock().unwrap();
            cancel_senders.insert(id, cancel_sender);

            let event_sender = self.event_sender.clone();
            tokio::spawn(async move {
                // Move the temporary file into this task so it will not be deleted
                // until after the process has started properly.
                let _worker_args = worker_args;
                event_sender
                    .send(Event {
                        id,
                        status: Status::Running,
                    })
                    .await
                    .expect("Failed to send Running event.");

                tokio::select! {
                    Ok(()) = cancel_receiver => {
                        child.kill().await.expect("Failed to kill child process.");
                        event_sender
                            .send(Event {
                                id,
                                status: Status::Cancelled,
                            })
                            .await
                            .expect("Failed to send Cancelled event.")
                    }
                    res = child.wait() => {
                        match res {
                            Ok(status) => {
                                if status.success() {
                                    event_sender
                                        .send(Event {
                                            id,
                                            status: Status::Complete,
                                        })
                                        .await
                                        .expect("Failed to send Complete event.")
                                } else {
                                    event_sender
                                        .send(Event {
                                            id,
                                            status: Status::Error,
                                        })
                                        .await
                                        .expect("Failed to send Error event.");
                                }
                            }
                            Err(err) => {
                                event_sender
                                    .send(Event {
                                        id,
                                        status: Status::Error,
                                    })
                                    .await
                                    .expect("Failed to send Error event.");
                            }
                        }
                    }
                }
            });
        }

        Ok(pids)
    }

    fn listen(&self) -> miette::Result<impl Stream<Item = Event<Self::ID>>> {
        // Explicit block to allow us to drop the MutexGuard after we
        // take the receiver.
        let channel = {
            let mut receiver = self
                .event_receiver
                .try_lock()
                .map_err(|err| miette!("Failed to listen: {}", err))?;

            let channel = receiver.take().ok_or(miette!(
                "Failed to listen: Executor is already being listened to."
            ))?;

            channel
        };
        Ok(stream::unfold(channel, |mut channel| async {
            channel.recv().await.map(|event| (event, channel))
        }))
    }

    fn reconnect(&self, task_ids: impl IntoIterator<Item = Self::ID>) {}

    fn cancel(&self, task_ids: impl IntoIterator<Item = Self::ID>) {
        let mut cancel_senders = self.cancel_senders.lock().unwrap();
        for task_id in task_ids.into_iter() {
            let cancel_sender = cancel_senders.remove(&task_id).unwrap();
            cancel_sender.send(()).unwrap();
        }
    }

    fn restart(&self, task_ids: impl IntoIterator<Item = Self::ID>) {}
}

#[cfg(test)]
mod tests {
    use futures::StreamExt;

    use super::*;

    // Test that we can list the available workers in $PATH
    #[tokio::test]
    async fn subprocess_workers() -> miette::Result<()> {
        let executor = SubprocessExecutor::new();

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

        dbg!(&workers);
        panic!();

        Ok(())
    }

    // Test that we can launch some events and listen for
    // their status changes.
    #[tokio::test]
    async fn execute_subprocess() -> miette::Result<()> {
        let task_plans = vec![TaskPlan {
            worker_name: "hello-world-worker".to_string(),
            task_name: "greet".to_string(),

            inputs: HashMap::new(),
            outputs: HashMap::new(),

            resources: SubprocessResourceSpec {},
            environment: SubprocessEnvironmentSpec {},
        }];
        let executor = SubprocessExecutor::new();

        let stream = executor.listen()?;
        executor.execute(task_plans).await?;

        let events = stream.take(2).collect::<Vec<_>>().await;
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].status, Status::Running);
        assert_eq!(events[1].status, Status::Complete);

        Ok(())
    }

    // Test that we can launch some events and listen for
    // their status changes even if we launch tasks before
    // we start listening to the executor.
    #[tokio::test]
    async fn execute_subprocess_execute_before_listen() -> miette::Result<()> {
        let task_plans = vec![TaskPlan {
            worker_name: "hello-world-worker".to_string(),
            task_name: "greet".to_string(),

            inputs: HashMap::new(),
            outputs: HashMap::new(),

            resources: SubprocessResourceSpec {},
            environment: SubprocessEnvironmentSpec {},
        }];
        let executor = SubprocessExecutor::new();

        executor.execute(task_plans).await?;
        let stream = executor.listen()?;

        let events = stream.take(2).collect::<Vec<_>>().await;
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].status, Status::Running);
        assert_eq!(events[1].status, Status::Complete);

        Ok(())
    }

    // Test that we can launch some events and listen for
    // errors when the occur
    #[tokio::test]
    async fn execute_subprocess_error() -> miette::Result<()> {
        let task_plans = vec![TaskPlan {
            worker_name: "error-worker".to_string(),
            task_name: "".to_string(),

            inputs: HashMap::new(),
            outputs: HashMap::new(),

            resources: SubprocessResourceSpec {},
            environment: SubprocessEnvironmentSpec {},
        }];
        let executor = SubprocessExecutor::new();

        let stream = executor.listen()?;
        executor.execute(task_plans).await?;

        let events = stream.take(2).collect::<Vec<_>>().await;
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].status, Status::Running);
        assert_eq!(events[1].status, Status::Error);

        Ok(())
    }

    // Test that we can launch some events and then cancel them
    // before they complete.
    #[tokio::test]
    async fn execute_subprocess_cancel() -> miette::Result<()> {
        let task_plans = vec![TaskPlan {
            worker_name: "error-worker".to_string(),
            task_name: "".to_string(),

            inputs: HashMap::new(),
            outputs: HashMap::new(),

            resources: SubprocessResourceSpec {},
            environment: SubprocessEnvironmentSpec {},
        }];
        let executor = SubprocessExecutor::new();

        let stream = executor.listen()?;
        let task_ids = executor.execute(task_plans).await?;
        executor.cancel(task_ids);

        let events = stream.take(2).collect::<Vec<_>>().await;
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].status, Status::Running);
        assert_eq!(events[1].status, Status::Cancelled);

        Ok(())
    }
}
