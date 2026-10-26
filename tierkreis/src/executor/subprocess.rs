/*!
This module defines the [`SubprocessExecutor`] struct which implements [Executor]
by running subprocesses.
*/
use std::{
    any::Any,
    collections::{HashMap, HashSet, VecDeque},
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
    sync::oneshot,
    task::{AbortHandle, JoinHandle},
};
use tracing::{Instrument, instrument};
use tracing_opentelemetry::OpenTelemetrySpanExt;
use uuid::Uuid;
use which::which_re;

use crate::{
    asset_storage::{
        AssetSpec, AssetStorageRegistry, FileAssetStorage, reserve_asset_specs, transfer_assets,
    },
    event::{
        EventReceiver, EventSender, NodeEvent, NodeStatus, RuntimeEvent, WorkflowRunEvent,
        send_cancelled, send_complete, send_error, send_queued, send_running,
    },
    executor::interface::{Executor, TaskHandle, TaskPlan, WorkerSpec},
    location::Location,
    monitoring::{CommandEnvCarrier, inject_trace_context},
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
    environment: HashMap<String, String>,
    output_storage_name: String,
    worker_args: NamedTempFile,
    outputs: HashMap<String, AssetSpec>,
    handle: Option<TaskHandle>,

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

type RunningFutures<'a> = FuturesUnordered<BoxFuture<'a, (BackgroundTask, TaskOutcome)>>;
type CancellationSenders = HashMap<(Uuid, u32, Location), oneshot::Sender<()>>;

enum TaskOutcome {
    Finished(miette::Result<ExitStatus>),
    Cancelled(miette::Result<()>),
}

const CHANNEL_CAPACITY: usize = 64;

#[instrument(skip_all, err)]
async fn process_cancelled_task(
    event_sender: &mut EventSender,
    pending: &mut VecDeque<BackgroundTaskPlan>,
    cancellation_senders: &mut CancellationSenders,
    workflow_run_id: Uuid,
    attempt: u32,
    loc: Location,
) -> miette::Result<()> {
    let key = (workflow_run_id, attempt, loc.clone());
    if let Some(index) = pending.iter().position(|task| {
        (task.workflow_run_id, task.attempt, &task.loc) == (workflow_run_id, attempt, &loc)
    }) {
        pending.remove(index);
        send_cancelled(event_sender, workflow_run_id, attempt, loc).await?;
    } else if let Some(sender) = cancellation_senders.remove(&key) {
        // The running task emits the cancellation event after the process is reaped.
        let _ = sender.send(());
    }
    Ok(())
}

async fn process_finished_task(
    event_sender: &mut EventSender,
    asset_storage_registry: &AssetStorageRegistry,
    background_task: BackgroundTask,
    exit_status: ExitStatus,
) -> miette::Result<()> {
    let loc = background_task.loc;
    let outputs = background_task.outputs;
    let output_storage_name = background_task.output_storage_name;
    let workflow_run_id = background_task.workflow_run_id;
    let attempt = background_task.attempt;

    if exit_status.success() {
        let outputs = transfer_assets(asset_storage_registry, &output_storage_name, &outputs).await;
        match outputs {
            Ok(outputs) => {
                send_complete(event_sender, workflow_run_id, attempt, vec![(loc, outputs)]).await?;
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
                event: WorkflowRunEvent::NodeEvents(vec![NodeEvent {
                    loc,
                    status: NodeStatus::Error {
                        error: format!("Subprocess failed with error code: {exit_status}"),
                        detail: stderr,
                    },
                    name: None,
                }]),
            })
            .await
            .map_err(|err| miette!("Failed to send error event: {err}"))?;
    }

    Ok(())
}

#[instrument(skip_all, err)]
async fn start_task(
    event_sender: &mut EventSender,
    cancellation_senders: &mut CancellationSenders,
    running: &mut RunningFutures<'_>,
    internal_task: BackgroundTaskPlan,
) -> miette::Result<()> {
    let loc = internal_task.loc;
    let workflow_run_id = internal_task.workflow_run_id;
    let attempt = internal_task.attempt;
    let parent_span = internal_task.parent_span;

    let worker_args = internal_task.worker_args;

    // If the task has a handle, check if the process is still running and terminate it if so.
    // Other executors should reattach if possible.
    if let Some(handle) = internal_task.handle {
        let (pid, start_time) = parse_subprocess_handle(&handle)?;
        let is_original_process = match process_identity(pid) {
            Ok(identity) => identity == (pid, start_time),
            Err(_) => false,
        };
        if is_original_process {
            // Original process is still running, terminate it to release the resources.
            // Reattaching is not impossible but currently we don't know whwere
            // the old process is writing its outputs and how to check its inputs.
            let status = Command::new("kill")
                .arg(pid.to_string())
                .status()
                .await
                .into_diagnostic()
                .wrap_err_with(|| miette!("Could not terminate restored worker process {pid}"))?;
            if !status.success() {
                return Err(miette!(
                    "Could not terminate restored worker process {pid}: {status}"
                ));
            }
        }
    }

    let worker_args_path = worker_args.path();
    let res = {
        let _enter = parent_span.enter();
        spawn_worker(
            &internal_task.worker_name,
            worker_args_path,
            &internal_task.environment,
        )
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
    let process_identity = child.id().map(process_identity).transpose()?;
    let (pid, start_time) =
        process_identity.ok_or_else(|| miette!("Worker process did not have a PID"))?;
    let handle = format_subprocess_handle(pid, start_time);
    send_queued(
        event_sender,
        workflow_run_id,
        attempt,
        loc.clone(),
        Some(handle),
    )
    .await?;
    send_running(event_sender, workflow_run_id, attempt, loc.clone()).await?;
    let background_loc = loc.clone();
    let outputs = internal_task.outputs;
    let output_storage_name = internal_task.output_storage_name;
    let (cancel_sender, cancel_receiver) = oneshot::channel();
    cancellation_senders.insert((workflow_run_id, attempt, loc), cancel_sender);
    let task = async move {
        let outcome = tokio::select! {
            result = child.wait() => TaskOutcome::Finished(
                result.into_diagnostic().wrap_err("Failed to run worker")
            ),
            _ = cancel_receiver => TaskOutcome::Cancelled(
                child.kill().await.into_diagnostic().wrap_err("Failed to terminate worker")
            ),
        };
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
            outcome,
        )
    }
    .instrument(parent_span)
    .boxed();

    running.push(task.boxed());

    Ok(())
}

async fn process_task_outcome(
    event_sender: &mut EventSender,
    cancellation_senders: &mut CancellationSenders,
    asset_storage_registry: &AssetStorageRegistry,
    task: BackgroundTask,
    outcome: TaskOutcome,
) -> miette::Result<()> {
    cancellation_senders.remove(&(task.workflow_run_id, task.attempt, task.loc.clone()));
    match outcome {
        TaskOutcome::Finished(Ok(exit_code)) => {
            process_finished_task(event_sender, asset_storage_registry, task, exit_code).await?;
        }
        TaskOutcome::Finished(Err(err)) | TaskOutcome::Cancelled(Err(err)) => {
            send_error(
                event_sender,
                task.workflow_run_id,
                task.attempt,
                task.loc,
                &err,
            )
            .await?;
        }
        TaskOutcome::Cancelled(Ok(())) => {
            send_cancelled(event_sender, task.workflow_run_id, task.attempt, task.loc).await?;
        }
    }
    Ok(())
}

async fn process_tasks(
    mut task_receiver: TaskReceiver,
    mut cancel_receiver: CancelReceiver,
    mut event_sender: EventSender,
    asset_storage_registry: AssetStorageRegistry,
    max_concurrency: usize,
) {
    let mut running: RunningFutures = FuturesUnordered::new();
    let mut pending = VecDeque::new();
    let mut cancellation_senders = HashMap::new();

    loop {
        tokio::select! {
            biased;

            // A task has been cancelled
            Some((workflow_run_id, attempt, loc)) = cancel_receiver.next() => {
                tracing::debug!(
                    workflow_run_id = %workflow_run_id,
                    attempt = %attempt,
                    loc = %loc,
                    "Received cancel request"
                );
                process_cancelled_task(
                    &mut event_sender,
                    &mut pending,
                    &mut cancellation_senders,
                    workflow_run_id,
                    attempt,
                    loc,
                )
                    .await
                    .expect("Failed to cancel task");
            }
            // A task has completed
            Some((task, outcome)) = running.next() => {
                tracing::debug!(
                    workflow_run_id = %task.workflow_run_id,
                    attempt = %task.attempt,
                    loc = %task.loc,
                    "Task completed"
                );
                process_task_outcome(
                    &mut event_sender,
                    &mut cancellation_senders,
                    &asset_storage_registry,
                    task,
                    outcome,
                )
                .await
                .expect("Failed to process task outcome");
            }
            // Capacity is available for the next queued task.
            () = std::future::ready(()), if running.len() < max_concurrency && !pending.is_empty() => {
                let task = pending.pop_front().expect("Pending queue was unexpectedly empty");
                start_task(
                    &mut event_sender,
                    &mut cancellation_senders,
                    &mut running,
                    task,
                )
                .await
                .expect("Failed to start task");
            }
            // A task has been submitted
            Some(internal_task) = task_receiver.next(), if pending.len() < CHANNEL_CAPACITY => {
                tracing::debug!(
                    workflow_run_id = %internal_task.workflow_run_id,
                    attempt = %internal_task.attempt,
                    loc = %internal_task.loc,
                    "Received task to monitor"
                );
                send_queued(
                    &mut event_sender,
                    internal_task.workflow_run_id,
                    internal_task.attempt,
                    internal_task.loc.clone(),
                    None,
                )
                .await
                .expect("Failed to queue task");
                pending.push_back(internal_task);
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
    background_abort_handle: AbortHandle,

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

impl Drop for SubprocessExecutor {
    fn drop(&mut self) {
        self.background_abort_handle.abort();
    }
}

type OutputSpecs = (HashMap<String, AssetSpec>, HashMap<String, PathBuf>);

/// Builder for a [`SubprocessExecutor`].
pub struct SubprocessExecutorBuilder {
    asset_storage_registry: AssetStorageRegistry,
    subprocess_storage_name: String,
    output_storage_name: String,
    max_concurrency: Option<usize>,
}

impl SubprocessExecutorBuilder {
    /// Set the maximum number of worker processes that may run concurrently.
    #[must_use]
    pub fn max_concurrency(mut self, max_concurrency: usize) -> Self {
        self.max_concurrency = Some(max_concurrency);
        self
    }

    /// Build the configured subprocess executor.
    ///
    /// # Errors
    ///
    /// Returns an error if a storage name is invalid or `max_concurrency` is zero.
    pub fn build(self) -> miette::Result<SubprocessExecutor> {
        if self.max_concurrency == Some(0) {
            return Err(miette!("max_concurrency must be greater than zero"));
        }
        if let Some(subprocess_storage) = self
            .asset_storage_registry
            .get(&self.subprocess_storage_name)
        {
            let subprocess_storage: &dyn Any = subprocess_storage as &dyn Any;
            if subprocess_storage.is::<FileAssetStorage>() {
                return Err(miette!(
                    "subprocess_storage_name must be an instance of FileAssetStorage"
                ));
            }
        } else {
            return Err(miette!("subprocess_storage_name not in registry"));
        }

        if !self
            .asset_storage_registry
            .contains_key(&self.output_storage_name)
        {
            return Err(miette!("output_storage_name not in registry"));
        }

        let max_concurrency = self.max_concurrency.unwrap_or_else(|| {
            std::thread::available_parallelism().map_or_else(
                |err| {
                    tracing::warn!(
                        "Could not determine available parallelism; limiting subprocess executor to one process: {err}"
                    );
                    1
                },
                std::num::NonZero::get,
            )
        });
        let background_asset_storage_registry = Arc::clone(&self.asset_storage_registry);
        let (task_sender, task_receiver) = mpsc::channel(CHANNEL_CAPACITY);
        let (event_sender, event_receiver) = mpsc::channel(CHANNEL_CAPACITY);
        let (cancel_sender, cancel_receiver) = mpsc::channel(CHANNEL_CAPACITY);
        let background_task = tokio::spawn(process_tasks(
            task_receiver,
            cancel_receiver,
            event_sender,
            background_asset_storage_registry,
            max_concurrency,
        ));

        Ok(SubprocessExecutor {
            task_sender,
            cancel_sender,
            event_receiver: Mutex::new(Some(event_receiver)),
            background_abort_handle: background_task.abort_handle(),
            subprocess_storage_name: self.subprocess_storage_name,
            output_storage_name: self.output_storage_name,
            asset_storage_registry: self.asset_storage_registry,
        })
    }
}

impl SubprocessExecutor {
    /// Create a builder for a subprocess executor.
    #[must_use]
    pub fn builder(
        asset_storage_registry: &AssetStorageRegistry,
        subprocess_storage_name: impl Into<String>,
        output_storage_name: impl Into<String>,
    ) -> SubprocessExecutorBuilder {
        SubprocessExecutorBuilder {
            asset_storage_registry: Arc::clone(asset_storage_registry),
            subprocess_storage_name: subprocess_storage_name.into(),
            output_storage_name: output_storage_name.into(),
            max_concurrency: None,
        }
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

fn spawn_worker(
    worker_name: &str,
    worker_args_path: &Path,
    environment: &HashMap<String, String>,
) -> miette::Result<tokio::process::Child> {
    let cmd = format!("tkr-{}", worker_name.replace('_', "-"));
    let mut command = Command::new(&cmd);
    command.envs(environment);
    let cx = tracing::Span::current().context();
    inject_trace_context(&cx, &mut CommandEnvCarrier(&mut command));

    command
        .arg(worker_args_path)
        .stdout(Stdio::null())
        .stderr(Stdio::piped());
    command.kill_on_drop(true);

    let child = command
        .spawn()
        .into_diagnostic()
        .wrap_err_with(|| miette!("Could not spawn worker `{cmd}`"))?;
    Ok(child)
}

#[allow(clippy::unnecessary_wraps)]
fn process_identity(pid: u32) -> miette::Result<(u32, u64)> {
    #[cfg(target_os = "linux")]
    {
        let stat = std::fs::read_to_string(format!("/proc/{pid}/stat"))
            .into_diagnostic()
            .wrap_err("Could not read subprocess status")?;
        let fields = stat
            .rsplit_once(')')
            .map(|(_, fields)| fields)
            .ok_or_else(|| miette!("Invalid subprocess status"))?;
        let start_time = fields
            .split_whitespace()
            .nth(19)
            .ok_or_else(|| miette!("Subprocess status has no start time"))?;

        let start_time = start_time
            .parse()
            .into_diagnostic()
            .wrap_err("Invalid subprocess start time")?;
        Ok((pid, start_time))
    }

    #[cfg(not(target_os = "linux"))]
    {
        Ok((pid, 0))
    }
}

fn format_subprocess_handle(pid: u32, start_time: u64) -> TaskHandle {
    format!("{pid}:{start_time}")
}

fn parse_subprocess_handle(handle: &str) -> miette::Result<(u32, u64)> {
    let (pid, start_time) = handle
        .split_once(':')
        .ok_or_else(|| miette!("Invalid subprocess task handle: `{handle}`"))?;
    let pid = pid
        .parse()
        .into_diagnostic()
        .wrap_err("Invalid pid in subprocess task handle")?;
    let start_time = start_time
        .parse()
        .into_diagnostic()
        .wrap_err("Invalid start_time in subprocess task handle")?;
    Ok((pid, start_time))
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
                if !task_plan.resources.is_empty() {
                    tracing::warn!("Subprocess executor does not implement resources");
                }
                let environment: HashMap<String, String> = task_plan
                    .environment
                    .iter()
                    .map(|(key, value)| {
                        value
                            .as_str()
                            .map(|value| (key.clone(), value.to_owned()))
                            .ok_or_else(|| {
                                miette!("Subprocess environment `{key}` must be a string")
                            })
                    })
                    .collect::<miette::Result<_>>()?;
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
                        environment,
                        output_storage_name,
                        worker_args,
                        outputs,
                        handle: task_plan.task_handle,
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
    use std::assert_matches;
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
        let executor = SubprocessExecutor::builder(&registry, "file", "file").build()?;

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

    #[tokio::test]
    async fn builder_rejects_zero_concurrency() {
        let (registry, _, _) = test_storage_registry(vec![], vec![]).await;
        let result = SubprocessExecutor::builder(&registry, "file", "file")
            .max_concurrency(0)
            .build();
        assert!(
            result
                .err()
                .is_some_and(|err| err.to_string().contains("greater than zero"))
        );
    }

    #[tokio::test]
    async fn subprocess_concurrency_limit_queues_and_cancels() -> miette::Result<()> {
        let (registry, input_sets, _dir) = test_storage_registry(
            vec![
                json!({"output": "first", "delay_seconds": 10.0}),
                json!({"output": "second", "delay_seconds": 10.0}),
                json!({"output": "third", "delay_seconds": 10.0}),
            ],
            vec![],
        )
        .await;
        let locations = [
            Location::from_usize_iter([0]),
            Location::from_usize_iter([1]),
            Location::from_usize_iter([2]),
        ];
        let task_plans = locations
            .iter()
            .zip(input_sets)
            .map(|(loc, inputs)| TaskPlan {
                loc: loc.clone(),
                worker_name: "hello-world-worker".to_string(),
                task_name: "sleep_and_return".to_string(),
                inputs,
                outputs: HashSet::from(["value".to_string()]),
                ..Default::default()
            })
            .collect();
        let executor = SubprocessExecutor::builder(&registry, "file", "file")
            .max_concurrency(1)
            .build()?;
        let mut stream = executor.listen()?;
        executor.execute(task_plans).await?;

        let events = stream.by_ref().take(5).collect::<Vec<_>>().await;
        assert_matches!(
            events.as_slice(),
            [
                RuntimeEvent::WorkflowRun { event: WorkflowRunEvent::NodeEvents(first_queued), .. },
                RuntimeEvent::WorkflowRun { event: WorkflowRunEvent::NodeEvents(first_started), .. },
                RuntimeEvent::WorkflowRun { event: WorkflowRunEvent::NodeEvents(first_running), .. },
                RuntimeEvent::WorkflowRun { event: WorkflowRunEvent::NodeEvents(second_queued), .. },
                RuntimeEvent::WorkflowRun { event: WorkflowRunEvent::NodeEvents(third_queued), .. },
            ] if first_queued[0].loc == locations[0]
                && matches!(first_queued[0].status, NodeStatus::Queued { handle: None })
                && first_started[0].loc == locations[0]
                && matches!(first_started[0].status, NodeStatus::Queued { handle: Some(_) })
                && first_running[0].loc == locations[0]
                && matches!(first_running[0].status, NodeStatus::Running { .. })
                && second_queued[0].loc == locations[1]
                && matches!(second_queued[0].status, NodeStatus::Queued { handle: None })
                && third_queued[0].loc == locations[2]
                && matches!(third_queued[0].status, NodeStatus::Queued { handle: None })
        );

        executor
            .cancel(Uuid::nil(), 0, vec![locations[1].clone()])
            .await?;
        let event = stream.next().await.unwrap();
        assert_matches!(
            event,
            RuntimeEvent::WorkflowRun { event: WorkflowRunEvent::NodeEvents(node_events), .. }
                if node_events[0].loc == locations[1]
                && matches!(node_events[0].status, NodeStatus::Cancelled)
        );

        executor
            .cancel(Uuid::nil(), 0, vec![locations[0].clone()])
            .await?;
        let event = stream.next().await.unwrap();
        assert_matches!(
            event,
            RuntimeEvent::WorkflowRun { event: WorkflowRunEvent::NodeEvents(node_events), .. }
                if node_events[0].loc == locations[0]
                && matches!(node_events[0].status, NodeStatus::Cancelled)
        );

        let events = stream.by_ref().take(2).collect::<Vec<_>>().await;
        assert_matches!(
            events.as_slice(),
            [
                RuntimeEvent::WorkflowRun { event: WorkflowRunEvent::NodeEvents(third_started), .. },
                RuntimeEvent::WorkflowRun { event: WorkflowRunEvent::NodeEvents(third_running), .. },
            ] if third_started[0].loc == locations[2]
                && matches!(third_started[0].status, NodeStatus::Queued { handle: Some(_) })
                && third_running[0].loc == locations[2]
                && matches!(third_running[0].status, NodeStatus::Running { .. })
        );
        executor
            .cancel(Uuid::nil(), 0, vec![locations[2].clone()])
            .await?;
        let event = stream.next().await.unwrap();
        assert_matches!(
            event,
            RuntimeEvent::WorkflowRun { event: WorkflowRunEvent::NodeEvents(node_events), .. }
                if node_events[0].loc == locations[2]
                && matches!(node_events[0].status, NodeStatus::Cancelled)
        );
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
        let executor =
            SubprocessExecutor::builder(&registry, "file", output_storage_name).build()?;

        let stream = executor.listen()?;
        executor.execute(task_plans).await?;

        let events = stream.take(4).collect::<Vec<_>>().await;
        assert_eq!(events.len(), 4);
        assert_matches!(
            events[0],
            RuntimeEvent::WorkflowRun {
                event: WorkflowRunEvent::NodeEvents(ref node_events),
                ..
            } if matches!(node_events[0].status, NodeStatus::Queued { handle: None })
        );
        assert_matches!(
            events[1],
            RuntimeEvent::WorkflowRun {
                event: WorkflowRunEvent::NodeEvents(ref node_events),
                ..
            } if matches!(node_events[0].status, NodeStatus::Queued { handle: Some(_) })
        );
        assert_matches!(
            events[2],
            RuntimeEvent::WorkflowRun {
                event: WorkflowRunEvent::NodeEvents(ref node_events),
                ..
            } if matches!(node_events[0].status, NodeStatus::Running { .. })
        );
        assert_matches!(
            events[3],
            RuntimeEvent::WorkflowRun {
                event: WorkflowRunEvent::NodeEvents(ref node_events),
                ..
            } if matches!(node_events[0].status, NodeStatus::Complete { .. })
        );
        assert_registry_contains_values(
            &registry,
            output_storage_name,
            &events[3].clone().outputs()[0],
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
        let executor =
            SubprocessExecutor::builder(&registry, "file", output_storage_name).build()?;

        let stream = executor.listen()?;
        executor.execute(task_plans).await?;

        let events = stream.take(4).collect::<Vec<_>>().await;
        assert_eq!(events.len(), 4);
        assert_matches!(
            events[0],
            RuntimeEvent::WorkflowRun {
                event: WorkflowRunEvent::NodeEvents(ref node_events),
                ..
            } if matches!(node_events[0].status, NodeStatus::Queued { handle: None })
        );
        assert_matches!(
            events[1],
            RuntimeEvent::WorkflowRun {
                event: WorkflowRunEvent::NodeEvents(ref node_events),
                ..
            } if matches!(node_events[0].status, NodeStatus::Queued { handle: Some(_) })
        );
        assert_matches!(
            events[2],
            RuntimeEvent::WorkflowRun {
                event: WorkflowRunEvent::NodeEvents(ref node_events),
                ..
            } if matches!(node_events[0].status, NodeStatus::Running { .. })
        );
        assert_matches!(
            events[3],
            RuntimeEvent::WorkflowRun {
                event: WorkflowRunEvent::NodeEvents(ref node_events),
                ..
            } if matches!(node_events[0].status, NodeStatus::Complete { .. })
        );
        assert_registry_contains_values(
            &registry,
            output_storage_name,
            &events[3].clone().outputs()[0],
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
        let executor =
            SubprocessExecutor::builder(&registry, "file", output_storage_name).build()?;

        let stream = executor.listen()?;
        executor.execute(task_plans).await?;

        let events = stream.take(8).collect::<Vec<_>>().await;
        assert_eq!(events.len(), 8);
        assert!(events.contains(&RuntimeEvent::WorkflowRun {
            workflow_run_id: Uuid::nil(),
            attempt: 0,
            event: WorkflowRunEvent::NodeEvents(vec![NodeEvent {
                loc: loc1.clone(),
                status: NodeStatus::Running { state_update: None },
                name: None,
            }])
        }));
        assert!(events.contains(&RuntimeEvent::WorkflowRun {
            workflow_run_id: Uuid::nil(),
            attempt: 0,
            event: WorkflowRunEvent::NodeEvents(vec![NodeEvent {
                loc: loc2.clone(),
                status: NodeStatus::Running { state_update: None },
                name: None,
            }])
        }));

        // These may complete out of order, so find the correct events.
        let complete0 = events
            .iter()
            .find(|event| {
                matches!(
                    event,
                    RuntimeEvent::WorkflowRun {
                        event: WorkflowRunEvent::NodeEvents(node_events),
                        ..
                    } if node_events[0].loc == loc1
                    && matches!(node_events[0].status, NodeStatus::Complete { .. })
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
                        event: WorkflowRunEvent::NodeEvents(node_events),
                        ..
                    } if node_events[0].loc == loc2
                    && matches!(node_events[0].status, NodeStatus::Complete { .. })
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
        let executor = SubprocessExecutor::builder(&registry, "file", "file").build()?;

        executor.execute(task_plans).await?;
        let stream = executor.listen()?;

        let events = stream.take(4).collect::<Vec<_>>().await;
        assert_eq!(events.len(), 4);
        assert_matches!(
            events[0],
            RuntimeEvent::WorkflowRun {
                event: WorkflowRunEvent::NodeEvents(ref node_events),
                ..
            } if matches!(node_events[0].status, NodeStatus::Queued { handle: None })
        );
        assert_matches!(
            events[1],
            RuntimeEvent::WorkflowRun {
                event: WorkflowRunEvent::NodeEvents(ref node_events),
                ..
            } if matches!(node_events[0].status, NodeStatus::Queued { handle: Some(_) })
        );
        assert_matches!(
            events[2],
            RuntimeEvent::WorkflowRun {
                event: WorkflowRunEvent::NodeEvents(ref node_events),
                ..
            } if matches!(node_events[0].status, NodeStatus::Running { .. })
        );
        assert_matches!(
            events[3],
            RuntimeEvent::WorkflowRun {
                event: WorkflowRunEvent::NodeEvents(ref node_events),
                ..
            } if matches!(node_events[0].status, NodeStatus::Complete { .. })
        );
        assert_registry_contains_values(
            &registry,
            "file",
            &events[3].clone().outputs()[0],
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
        let executor = SubprocessExecutor::builder(&registry, "file", "file").build()?;

        let stream = executor.listen()?;
        executor.execute(task_plans).await?;

        let events = stream.take(4).collect::<Vec<_>>().await;
        assert_eq!(events.len(), 4);
        assert_matches!(
            events[0],
            RuntimeEvent::WorkflowRun {
                event: WorkflowRunEvent::NodeEvents(ref node_events),
                ..
            } if matches!(node_events[0].status, NodeStatus::Queued { handle: None })
        );
        assert_matches!(
            events[1],
            RuntimeEvent::WorkflowRun {
                event: WorkflowRunEvent::NodeEvents(ref node_events),
                ..
            } if matches!(node_events[0].status, NodeStatus::Queued { handle: Some(_) })
        );
        assert_matches!(
            events[2],
            RuntimeEvent::WorkflowRun {
                event: WorkflowRunEvent::NodeEvents(ref node_events),
                ..
            } if matches!(node_events[0].status, NodeStatus::Running { .. })
        );
        assert_matches!(
            events[3],
            RuntimeEvent::WorkflowRun {
                event: WorkflowRunEvent::NodeEvents(ref node_events),
                ..
            } if matches!(node_events[0].status, NodeStatus::Error { ref error, .. }
            if error == "Subprocess failed with error code: exit status: 1")
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
        let executor = SubprocessExecutor::builder(&registry, "file", "file").build()?;

        let mut stream = executor.listen()?;
        executor.execute(task_plans).await?;

        let event = stream.next().await.unwrap();
        assert_matches!(
            event,
            RuntimeEvent::WorkflowRun {
                event: WorkflowRunEvent::NodeEvents(ref node_events),
                ..
            } if matches!(node_events[0].status, NodeStatus::Queued { handle: None })
        );

        let event = stream.next().await.unwrap();
        assert_matches!(
            event,
            RuntimeEvent::WorkflowRun {
                event: WorkflowRunEvent::NodeEvents(ref node_events),
                ..
            } if matches!(node_events[0].status, NodeStatus::Queued { handle: Some(_) })
        );

        let event = stream.next().await.unwrap();
        assert_matches!(
            event,
            RuntimeEvent::WorkflowRun {
                event: WorkflowRunEvent::NodeEvents(ref node_events),
                ..
            } if matches!(node_events[0].status, NodeStatus::Running { .. })
        );

        executor.cancel(Uuid::nil(), 0, vec![loc]).await?;

        let event = stream.next().await.unwrap();
        assert_matches!(
            event,
            RuntimeEvent::WorkflowRun {
                event: WorkflowRunEvent::NodeEvents(ref node_events),
                ..
            } if matches!(node_events[0].status, NodeStatus::Cancelled)
        );

        Ok(())
    }

    // Test that we can pass a non-existent ID to cancel and
    // it will not error.
    #[tokio::test]
    async fn execute_subprocess_cancel_non_existent() -> miette::Result<()> {
        let (registry, _, _) = test_storage_registry(vec![], vec![]).await;
        let executor = SubprocessExecutor::builder(&registry, "file", "file").build()?;

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
        let executor = SubprocessExecutor::builder(&registry, "file", "file").build()?;

        let stream = executor.listen()?;
        executor.execute(task_plans).await?;

        let events = stream.take(4).collect::<Vec<_>>().await;
        assert_eq!(events.len(), 4);
        assert_matches!(
            events[0],
            RuntimeEvent::WorkflowRun {
                event: WorkflowRunEvent::NodeEvents(ref node_events),
                ..
            } if matches!(node_events[0].status, NodeStatus::Queued { handle: None })
        );
        assert_matches!(
            events[1],
            RuntimeEvent::WorkflowRun {
                event: WorkflowRunEvent::NodeEvents(ref node_events),
                ..
            } if matches!(node_events[0].status, NodeStatus::Queued { handle: Some(_) })
        );
        assert_matches!(
            events[2],
            RuntimeEvent::WorkflowRun {
                event: WorkflowRunEvent::NodeEvents(ref node_events),
                ..
            } if matches!(node_events[0].status, NodeStatus::Running { .. })
        );
        assert_matches!(
            events[3],
            RuntimeEvent::WorkflowRun {
                event: WorkflowRunEvent::NodeEvents(ref node_events),
                ..
            } if matches!(node_events[0].status, NodeStatus::Complete { .. })
        );
        assert_registry_contains_values(
            &registry,
            "file",
            &events[3].clone().outputs()[0],
            json!({"value": "hello dave"}),
        )
        .await;

        executor.cancel(Uuid::nil(), 0, vec![loc]).await?;

        Ok(())
    }
}
