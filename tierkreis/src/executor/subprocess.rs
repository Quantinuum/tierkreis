/*!
This module defines the [`SubprocessExecutor`] struct which implements [Executor]
by running subprocesses.
*/
use std::{
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
    process::{ExitStatus, Stdio},
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
use tokio::{
    io::AsyncReadExt,
    process::Command,
    task::{AbortHandle, JoinHandle},
};
use tracing::{Instrument, instrument};
use tracing_opentelemetry::OpenTelemetrySpanExt;
use uuid::Uuid;
use which::which_re;

use crate::{
    asset_storage::{
        AssetKind, AssetSpec, AssetStorageRegistry, reserve_asset_specs, transfer_assets,
    },
    event::{
        EventReceiver, EventSender, NodeEvent, NodeStatus, RuntimeEvent, WorkflowRunEvent,
        send_cancelled, send_complete, send_error, send_running,
    },
    executor::interface::{Executor, TaskPlan, WorkerSpec},
    location::Location,
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
    workflow_run_id: Uuid,
    attempt: u32,
    loc: Location,
    worker_name: String,
    output_storage_name: String,
    worker_args: NamedTempFile,
    outputs: HashMap<String, AssetSpec>,

    parent_span: tracing::Span,
}

struct BackgroundTask {
    workflow_run_id: Uuid,
    attempt: u32,
    loc: Location,
    output_storage_name: String,
    outputs: HashMap<String, AssetSpec>,
    stderr: JoinHandle<String>,

    // Handles to temporary files to prevent deletion
    // until the task is complete.
    _worker_args: NamedTempFile,
}

type TaskSender = mpsc::Sender<BackgroundTaskPlan>;
type TaskReceiver = mpsc::Receiver<BackgroundTaskPlan>;
type CancelSender = mpsc::Sender<(Uuid, u32, Location)>;
type CancelReceiver = mpsc::Receiver<(Uuid, u32, Location)>;

type RunningFutures<'a> =
    FuturesUnordered<BoxFuture<'a, (BackgroundTask, miette::Result<ExitStatus>)>>;
type AbortHandles = HashMap<(Uuid, u32, Location), AbortHandle>;

#[instrument(skip_all, err)]
async fn process_cancelled_task(
    event_sender: &mut EventSender,
    abort_handles: &mut AbortHandles,
    workflow_run_id: Uuid,
    attempt: u32,
    loc: Location,
) -> miette::Result<()> {
    let handle = abort_handles.remove(&(workflow_run_id, attempt, loc.clone()));
    if let Some(handle) = handle {
        handle.abort();
        send_cancelled(event_sender, workflow_run_id, attempt, loc).await?;
    }
    Ok(())
}

async fn process_finished_task(
    event_sender: &mut EventSender,
    abort_handles: &mut AbortHandles,
    asset_storage_registry: &AssetStorageRegistry,
    background_task: BackgroundTask,
    exit_status: ExitStatus,
) -> miette::Result<()> {
    let loc = background_task.loc;
    let outputs = background_task.outputs;
    let output_storage_name = background_task.output_storage_name;
    let workflow_run_id = background_task.workflow_run_id;
    let attempt = background_task.attempt;

    abort_handles.remove(&(workflow_run_id, attempt, loc.clone()));

    if exit_status.success() {
        let outputs = transfer_assets(asset_storage_registry, &output_storage_name, &outputs).await;
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
    } else {
        let stderr = background_task.stderr.await.ok();
        event_sender
            .send(RuntimeEvent::WorkflowRun {
                workflow_run_id,
                attempt,
                event: WorkflowRunEvent::NodeEvent(NodeEvent {
                    locs: vec![loc],
                    status: NodeStatus::Error {
                        error: format!("Subprocess failed with error code: {exit_status}"),
                        detail: stderr,
                    },
                }),
            })
            .await
            .map_err(|err| miette!("Failed to send error event: {err}"))?;
    }

    Ok(())
}

#[instrument(skip_all, err)]
async fn start_task(
    event_sender: &mut EventSender,
    abort_handles: &mut AbortHandles,
    running: &mut RunningFutures<'_>,
    internal_task: BackgroundTaskPlan,
) -> miette::Result<()> {
    let loc = internal_task.loc;
    let workflow_run_id = internal_task.workflow_run_id;
    let attempt = internal_task.attempt;
    let parent_span = internal_task.parent_span;
    send_running(event_sender, workflow_run_id, attempt, loc.clone()).await?;

    let worker_args = internal_task.worker_args;
    let worker_args_path = worker_args.path();
    let res = {
        let _enter = parent_span.enter();
        spawn_worker(&internal_task.worker_name, worker_args_path)
    };
    let mut child = match res {
        Ok(child) => child,
        Err(err) => {
            tracing::debug!(workflow_run_id = %workflow_run_id, attempt, loc = %loc, "Failed to spawn worker: {err}");
            send_error(event_sender, workflow_run_id, attempt, loc, &err).await?;
            return Ok(());
        }
    };
    let stderr = read_stderr(&mut child);
    let background_loc = loc.clone();
    let outputs = internal_task.outputs;
    let output_storage_name = internal_task.output_storage_name;
    let task = tokio::task::spawn(async move { child.wait().instrument(parent_span).await });

    abort_handles.insert((workflow_run_id, attempt, loc), task.abort_handle());
    let task = task
        .map(move |result| {
            (
                BackgroundTask {
                    workflow_run_id,
                    attempt,
                    loc: background_loc,
                    output_storage_name,
                    outputs,
                    stderr,
                    _worker_args: worker_args,
                },
                result
                    .map(|res| res.map_err(|err| miette!("Failed to run worker: {err}")))
                    .map_err(|err| miette!("Failed to join task: {err}"))
                    .flatten(),
            )
        })
        .boxed();

    running.push(task.boxed());

    Ok(())
}

async fn process_tasks(
    mut task_receiver: TaskReceiver,
    mut cancel_receiver: CancelReceiver,
    mut event_sender: EventSender,
    asset_storage_registry: AssetStorageRegistry,
) {
    let mut abort_handles: AbortHandles = HashMap::new();
    let mut running: RunningFutures = FuturesUnordered::new();

    loop {
        tokio::select! {
            // A task has been cancelled
            Some((workflow_run_id, attempt, loc)) = cancel_receiver.next() => {
                tracing::debug!(
                    workflow_run_id = %workflow_run_id,
                    attempt = %attempt,
                    loc = %loc,
                    "Received cancel request"
                );
                process_cancelled_task(&mut event_sender, &mut abort_handles, workflow_run_id, attempt, loc)
                    .await
                    .expect("Failed to cancel task");
            }
            // A task has completed
            Some((task, result)) = running.next() => {
                tracing::debug!(
                    workflow_run_id = %task.workflow_run_id,
                    attempt = %task.attempt,
                    loc = %task.loc,
                    "Task completed"
                );
                let exit_code = match result {
                    Ok(exit_code) => exit_code,
                    Err(err) => {
                        send_error(
                            &mut event_sender,
                            task.workflow_run_id,
                            task.attempt,
                            task.loc.clone(),
                            &err,
                        )
                        .await
                        .expect("Failed to send error event");
                        continue
                    }
                };

                process_finished_task(
                    &mut event_sender,
                    &mut abort_handles,
                    &asset_storage_registry,
                    task,
                    exit_code,
                )
                .await
                .expect("Failed to complete task");
            }
            // A task has been submitted
            Some(internal_task) = task_receiver.next() => {
                tracing::debug!(
                    workflow_run_id = %internal_task.workflow_run_id,
                    attempt = %internal_task.attempt,
                    loc = %internal_task.loc,
                    "Received task to monitor"
                );
                start_task(
                    &mut event_sender,
                    &mut abort_handles,
                    &mut running,
                    internal_task,
                )
                .await
                .expect("Failed to start task");
            }
            else => break
        }
    }
}

/// [`SubprocessExecutor`] defines an [Executor] that performs Task Nodes using Worker subprocesses.
pub struct SubprocessExecutor {
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
    pub async fn try_new(
        asset_storage_registry: &AssetStorageRegistry,
        subprocess_storage_name: &str,
        output_storage_name: &str,
    ) -> miette::Result<Self> {
        let asset_storage_registry_lock = asset_storage_registry.read().await;
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

    async fn build_inputs(
        &self,
        inputs: &HashMap<String, AssetSpec>,
    ) -> Result<HashMap<String, PathBuf>, miette::Error> {
        let inputs = transfer_assets(
            &self.asset_storage_registry,
            &self.subprocess_storage_name,
            inputs,
        )
        .await?;
        let inputs =
            write_input_paths(&inputs).wrap_err("Failed to collect Worker input filepaths")?;
        Ok(inputs)
    }

    async fn build_outputs(&self, outputs: HashSet<String>) -> Result<OutputSpecs, miette::Error> {
        let output_specs = reserve_asset_specs(
            &self.asset_storage_registry,
            &self.subprocess_storage_name,
            outputs.len(),
        )
        .await?;
        let outputs: HashMap<String, AssetSpec> = outputs.into_iter().zip(output_specs).collect();
        let output_paths = outputs
            .iter()
            .map(|(k, v)| Ok((k.clone(), v.path()?)))
            .collect::<Result<HashMap<_, _>, miette::Error>>()?;
        Ok((outputs, output_paths))
    }
}

struct CommandEnvCarrier<'a>(&'a mut Command);
impl opentelemetry::propagation::Injector for CommandEnvCarrier<'_> {
    fn set(&mut self, key: &str, value: String) {
        self.0.env(key.to_uppercase().replace('_', "-"), value);
    }
}

fn spawn_worker(
    worker_name: &str,
    worker_args_path: &Path,
) -> miette::Result<tokio::process::Child> {
    let cmd = format!("tkr-{}", worker_name.replace('_', "-"));
    let mut command = Command::new(&cmd);
    let cx = tracing::Span::current().context();
    opentelemetry::global::get_text_map_propagator(|p| {
        p.inject_context(&cx, &mut CommandEnvCarrier(&mut command));
    });

    command
        .arg(worker_args_path)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let child = command
        .spawn()
        .into_diagnostic()
        .wrap_err_with(|| miette!("Could not spawn worker `{cmd}`"))?;
    Ok(child)
}

fn read_stderr(child: &mut tokio::process::Child) -> tokio::task::JoinHandle<String> {
    let mut stderr = child.stderr.take().unwrap();
    tokio::spawn(async move {
        let mut stderr_out = String::new();
        // TODO: Redirect child output to file or db
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

    fn execute(&self, task_plans: Vec<TaskPlan>) -> BoxFuture<'_, miette::Result<()>> {
        let fut = async {
            let mut task_sender = self.task_sender.clone();

            for task_plan in task_plans {
                let inputs = self
                    .build_inputs(&task_plan.inputs)
                    .await
                    .wrap_err("Failed to build task inputs")?;
                let (outputs, output_paths) = self
                    .build_outputs(task_plan.outputs)
                    .await
                    .wrap_err("Failed to build task outputs")?;

                let worker_args = NamedTempFile::new()
                    .into_diagnostic()
                    .wrap_err("Failed to create worker call args file.")?;

                // Redirect the done_file to a temporary file as we
                // do not need it to figure out if a process has
                // completed currently.
                let done_file = NamedTempFile::new()
                    .into_diagnostic()
                    .wrap_err("Failed to create `done` file")?;

                let error_file = NamedTempFile::new()
                    .into_diagnostic()
                    .wrap_err("Failed to create `error` file")?;

                serde_json::to_writer(
                    &worker_args,
                    &WorkerCallArgs {
                        function_name: task_plan.task_name,
                        inputs,
                        outputs: output_paths,
                        error_path: error_file.into_temp_path().to_path_buf(),
                        done_path: done_file.into_temp_path().to_path_buf(),
                        ..Default::default()
                    },
                )
                .into_diagnostic()
                .wrap_err("Failed to write worker call args")?;

                let output_storage_name = task_plan
                    .output_storage_name
                    .clone()
                    .unwrap_or_else(|| self.output_storage_name.clone());

                task_sender
                    .send(BackgroundTaskPlan {
                        workflow_run_id: task_plan.workflow_run_id,
                        attempt: task_plan.attempt,
                        loc: task_plan.loc,
                        worker_name: task_plan.worker_name,
                        output_storage_name,
                        worker_args,
                        outputs,
                        parent_span: tracing::Span::current(),
                    })
                    .await
                    .into_diagnostic()
                    .wrap_err("Failed to enqueue background task.")?;
            }

            Ok(())
        };
        fut.boxed()
    }

    fn listen(&self) -> miette::Result<BoxStream<'static, RuntimeEvent>> {
        // Explicit block to allow us to drop the MutexGuard after we
        // take the receiver.
        let channel = {
            let mut receiver = self
                .event_receiver
                .try_lock()
                .map_err(|err| miette!("Failed to listen: {}", err))?;

            receiver.take().ok_or_else(|| {
                miette!("Failed to listen: Executor is already being listened to.")
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
                    .into_diagnostic()?;
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
        let (registry, _, _) = test_storage_registry(vec![], vec![]).await;
        let executor = SubprocessExecutor::try_new(&registry, "file", "file").await?;

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
        )
        .await;
        let mut outputs = HashSet::new();
        outputs.insert("value".to_string());

        let task_plans = vec![TaskPlan {
            worker_name: "hello-world-worker".to_string(),
            task_name: "greet".to_string(),

            inputs: input_sets[0].clone(),
            outputs,
            ..Default::default()
        }];
        let executor = SubprocessExecutor::try_new(&registry, "file", output_storage_name).await?;

        let stream = executor.listen()?;
        executor.execute(task_plans).await?;

        let events = stream.take(2).collect::<Vec<_>>().await;
        assert_eq!(events.len(), 2);
        assert!(matches!(
            events[0],
            RuntimeEvent::WorkflowRun {
                event: WorkflowRunEvent::NodeEvent(NodeEvent {
                    status: NodeStatus::Running { .. },
                    ..
                }),
                ..
            }
        ));
        assert!(matches!(
            events[1],
            RuntimeEvent::WorkflowRun {
                event: WorkflowRunEvent::NodeEvent(NodeEvent {
                    status: NodeStatus::Complete { .. },
                    ..
                }),
                ..
            }
        ));
        assert_registry_contains_values(
            &registry,
            output_storage_name,
            &events[1].clone().outputs()[0],
            json!({"value": "hello dave"}),
        )
        .await;

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
        )
        .await;
        let mut outputs = HashSet::new();
        outputs.insert("value".to_string());

        let task_plans = vec![TaskPlan {
            worker_name: "hello-world-worker".to_string(),
            task_name: "greet".to_string(),

            inputs: input_sets[0].clone(),
            outputs,

            ..Default::default()
        }];
        let executor = SubprocessExecutor::try_new(&registry, "file", output_storage_name).await?;

        let stream = executor.listen()?;
        executor.execute(task_plans).await?;

        let events = stream.take(2).collect::<Vec<_>>().await;
        assert_eq!(events.len(), 2);
        assert!(matches!(
            events[0],
            RuntimeEvent::WorkflowRun {
                event: WorkflowRunEvent::NodeEvent(NodeEvent {
                    status: NodeStatus::Running { .. },
                    ..
                }),
                ..
            }
        ));
        assert!(matches!(
            events[1],
            RuntimeEvent::WorkflowRun {
                event: WorkflowRunEvent::NodeEvent(NodeEvent {
                    status: NodeStatus::Complete { .. },
                    ..
                }),
                ..
            }
        ));
        assert_registry_contains_values(
            &registry,
            output_storage_name,
            &events[1].clone().outputs()[0],
            json!({"value": "hello dave"}),
        )
        .await;

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
        )
        .await;
        let mut outputs = HashSet::new();
        outputs.insert("value".to_string());

        let loc1 = Location::from_usize_iter([0]);
        let loc2 = Location::from_usize_iter([1]);
        let task_plans = vec![
            TaskPlan {
                loc: loc1.clone(),
                worker_name: "hello-world-worker".to_string(),
                task_name: "greet".to_string(),

                inputs: input_sets[0].clone(),
                outputs: outputs.clone(),

                ..Default::default()
            },
            TaskPlan {
                loc: loc2.clone(),
                worker_name: "hello-world-worker".to_string(),
                task_name: "greet".to_string(),

                inputs: input_sets[1].clone(),
                outputs: outputs.clone(),

                ..Default::default()
            },
        ];
        let executor = SubprocessExecutor::try_new(&registry, "file", output_storage_name).await?;

        let stream = executor.listen()?;
        executor.execute(task_plans).await?;

        let events = stream.take(4).collect::<Vec<_>>().await;
        dbg!(&events);
        assert_eq!(events.len(), 4);
        assert!(events.contains(&RuntimeEvent::WorkflowRun {
            workflow_run_id: Uuid::nil(),
            attempt: 0,
            event: WorkflowRunEvent::NodeEvent(NodeEvent {
                locs: vec![loc1.clone()],
                status: NodeStatus::Running { state_update: None }
            })
        }));
        assert!(events.contains(&RuntimeEvent::WorkflowRun {
            workflow_run_id: Uuid::nil(),
            attempt: 0,
            event: WorkflowRunEvent::NodeEvent(NodeEvent {
                locs: vec![loc2.clone()],
                status: NodeStatus::Running { state_update: None }
            })
        }));

        // These may complete out of order, so find the correct events.
        let complete0 = events
            .iter()
            .find(|event| {
                matches!(
                    event,
                    RuntimeEvent::WorkflowRun {
                        event: WorkflowRunEvent::NodeEvent(NodeEvent {
                            locs,
                            status: NodeStatus::Complete { .. }
                        }), ..
                    } if locs == &vec![loc1.clone()]
                )
            })
            .unwrap();
        assert_registry_contains_values(
            &registry,
            output_storage_name,
            &complete0.clone().outputs()[0],
            json!({"value": "hello dave"}),
        )
        .await;
        let complete1 = events
            .iter()
            .find(|event| {
                matches!(
                    event,
                    RuntimeEvent::WorkflowRun {
                        event: WorkflowRunEvent::NodeEvent(NodeEvent {
                            locs,
                            status: NodeStatus::Complete { .. }
                        }), ..
                    } if locs == &vec![loc2.clone()]
                )
            })
            .unwrap();
        assert_registry_contains_values(
            &registry,
            output_storage_name,
            &complete1.clone().outputs()[0],
            json!({"value": "hi steve"}),
        )
        .await;

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
        )
        .await;
        let mut outputs = HashSet::new();
        outputs.insert("value".to_string());

        let task_plans = vec![TaskPlan {
            worker_name: "hello-world-worker".to_string(),
            task_name: "greet".to_string(),

            inputs: input_sets[0].clone(),
            outputs: outputs.clone(),

            ..Default::default()
        }];
        let executor = SubprocessExecutor::try_new(&registry, "file", "file").await?;

        executor.execute(task_plans).await?;
        let stream = executor.listen()?;

        let events = stream.take(2).collect::<Vec<_>>().await;
        assert_eq!(events.len(), 2);
        assert!(matches!(
            events[0],
            RuntimeEvent::WorkflowRun {
                event: WorkflowRunEvent::NodeEvent(NodeEvent {
                    status: NodeStatus::Running { .. },
                    ..
                }),
                ..
            }
        ));
        assert!(matches!(
            events[1],
            RuntimeEvent::WorkflowRun {
                event: WorkflowRunEvent::NodeEvent(NodeEvent {
                    status: NodeStatus::Complete { .. },
                    ..
                }),
                ..
            }
        ));
        assert_registry_contains_values(
            &registry,
            "file",
            &events[1].clone().outputs()[0],
            json!({"value": "hello dave"}),
        )
        .await;

        Ok(())
    }

    // Test that we can launch a task and listen for
    // errors when they occur
    #[tokio::test]
    async fn execute_subprocess_error() -> miette::Result<()> {
        let (registry, _, _dir) = test_storage_registry(vec![], vec![]).await;
        let task_plans = vec![TaskPlan {
            worker_name: "hello-world-worker".to_string(),
            // hello-world-worker has no task called "hail"
            task_name: "hail".to_string(),

            inputs: HashMap::new(),

            ..Default::default()
        }];
        let executor = SubprocessExecutor::try_new(&registry, "file", "file").await?;

        let stream = executor.listen()?;
        executor.execute(task_plans).await?;

        let events = stream.take(2).collect::<Vec<_>>().await;
        assert_eq!(events.len(), 2);
        assert!(matches!(
            events[0],
            RuntimeEvent::WorkflowRun {
                event: WorkflowRunEvent::NodeEvent(NodeEvent {
                    status: NodeStatus::Running { .. },
                    ..
                }),
                ..
            }
        ));
        dbg!(&events[1]);
        assert!(matches!(
            events[1],
            RuntimeEvent::WorkflowRun {
                event: WorkflowRunEvent::NodeEvent(NodeEvent {
                    status: NodeStatus::Error { ref error, .. },
                    ..
                }),
                ..
            } if error == "Subprocess failed with error code: exit status: 1"
        ));

        Ok(())
    }

    // Test that we can launch a task and then cancel it
    // before it completes.
    #[tokio::test]
    async fn execute_subprocess_cancel() -> miette::Result<()> {
        let (registry, input_sets, _dir) = test_storage_registry(
            vec![json!({"greeting": "hello ", "subject": "dave"})],
            vec![],
        )
        .await;
        let mut outputs = HashSet::new();
        outputs.insert("value".to_string());

        let loc = Location::from_usize_iter([0]);
        let task_plans = vec![TaskPlan {
            loc: loc.clone(),
            worker_name: "hello-world-worker".to_string(),
            task_name: "greet".to_string(),

            inputs: input_sets[0].clone(),
            outputs: outputs.clone(),

            ..Default::default()
        }];
        let executor = SubprocessExecutor::try_new(&registry, "file", "file").await?;

        let mut stream = executor.listen()?;
        executor.execute(task_plans).await?;

        let event = stream.next().await.unwrap();
        assert!(matches!(
            event,
            RuntimeEvent::WorkflowRun {
                event: WorkflowRunEvent::NodeEvent(NodeEvent {
                    status: NodeStatus::Running { .. },
                    ..
                }),
                ..
            }
        ));

        executor.cancel(Uuid::nil(), 0, vec![loc]).await?;

        let event = stream.next().await.unwrap();
        assert!(matches!(
            event,
            RuntimeEvent::WorkflowRun {
                event: WorkflowRunEvent::NodeEvent(NodeEvent {
                    status: NodeStatus::Cancelled,
                    ..
                }),
                ..
            }
        ));

        Ok(())
    }

    // Test that we can pass a non-existent ID to cancel and
    // it will not error.
    #[tokio::test]
    async fn execute_subprocess_cancel_non_existent() -> miette::Result<()> {
        let (registry, _, _) = test_storage_registry(vec![], vec![]).await;
        let executor = SubprocessExecutor::try_new(&registry, "file", "file").await?;

        let loc = Location::from_usize_iter([0]);
        executor.cancel(Uuid::nil(), 0, vec![loc]).await?;

        Ok(())
    }

    // Test that we can pass a completed ID to cancel and
    // it will not error.
    #[tokio::test]
    async fn execute_subprocess_cancel_completed() -> miette::Result<()> {
        let (registry, input_sets, _dir) = test_storage_registry(
            vec![json!({"greeting": "hello ", "subject": "dave"})],
            vec![],
        )
        .await;
        let mut outputs = HashSet::new();
        outputs.insert("value".to_string());

        let loc = Location::from_usize_iter([0]);
        let task_plans = vec![TaskPlan {
            loc: loc.clone(),
            worker_name: "hello-world-worker".to_string(),
            task_name: "greet".to_string(),

            inputs: input_sets[0].clone(),
            outputs: outputs.clone(),

            ..Default::default()
        }];
        let executor = SubprocessExecutor::try_new(&registry, "file", "file").await?;

        let stream = executor.listen()?;
        executor.execute(task_plans).await?;

        let events = stream.take(2).collect::<Vec<_>>().await;
        assert_eq!(events.len(), 2);
        assert!(matches!(
            events[0],
            RuntimeEvent::WorkflowRun {
                event: WorkflowRunEvent::NodeEvent(NodeEvent {
                    status: NodeStatus::Running { .. },
                    ..
                }),
                ..
            }
        ));
        assert!(matches!(
            events[1],
            RuntimeEvent::WorkflowRun {
                event: WorkflowRunEvent::NodeEvent(NodeEvent {
                    status: NodeStatus::Complete { .. },
                    ..
                }),
                ..
            }
        ));
        assert_registry_contains_values(
            &registry,
            "file",
            &events[1].clone().outputs()[0],
            json!({"value": "hello dave"}),
        )
        .await;

        executor.cancel(Uuid::nil(), 0, vec![loc]).await?;

        Ok(())
    }
}
