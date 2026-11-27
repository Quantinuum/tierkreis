/*!
This module defines the [`SubprocessExecutor`] struct which implements [Executor]
by running subprocesses.
*/
use std::{
    any::Any,
    collections::{HashMap, HashSet, VecDeque},
    num::NonZero,
    path::{Path, PathBuf},
    process::{ExitStatus, Stdio},
    sync::{Arc, Mutex},
    time::Duration,
};

use futures::{
    FutureExt, SinkExt, StreamExt,
    channel::mpsc,
    future::BoxFuture,
    stream::{BoxStream, FuturesUnordered},
};
use miette::{Context, IntoDiagnostic, miette};
use regex::Regex;
use tempfile::NamedTempFile;
use tokio::{
    io::{AsyncRead, AsyncReadExt},
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
    executor::{
        commands::{WorkerCommands, validate_worker_commands, worker_invocation},
        interface::{Executor, TaskHandle, TaskPlan, WorkerSpec},
    },
    location::Location,
    monitoring::{CommandEnvCarrier, inject_trace_context},
    worker::WorkerCallArgs,
    worker_logs::{CHUNK_BYTES, LogSource, TaskLogSink, TaskLogWriter, complete_utf8_prefix},
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
    logs: Option<TaskLogWriter>,

    parent_span: tracing::Span,
}

struct BackgroundTask {
    workflow_run_id: Uuid,
    attempt: u32,
    loc: Location,
    output_storage_name: String,
    outputs: HashMap<String, AssetSpec>,
    stderr: Option<String>,

    // Handles to temporary files to prevent deletion
    // until the task is complete.
    _worker_args: NamedTempFile,
}

type TaskSender = mpsc::Sender<BackgroundTaskPlan>;
type TaskReceiver = mpsc::Receiver<BackgroundTaskPlan>;
type CancelSender = mpsc::Sender<(Uuid, u32, Location)>;
type CancelReceiver = mpsc::Receiver<(Uuid, u32, Location)>;

type RunningFutures<'a> =
    FuturesUnordered<BoxFuture<'a, (BackgroundTask, miette::Result<Option<ExitStatus>>)>>;
type RunningCancellations = HashMap<(Uuid, u32, Location), oneshot::Sender<()>>;

#[instrument(skip_all, err)]
async fn process_cancelled_task(
    event_sender: &mut EventSender,
    running_cancellations: &mut RunningCancellations,
    pending: &mut VecDeque<BackgroundTaskPlan>,
    workflow_run_id: Uuid,
    attempt: u32,
    loc: Location,
) -> miette::Result<()> {
    let handle = running_cancellations.remove(&(workflow_run_id, attempt, loc.clone()));
    if let Some(handle) = handle {
        let _ = handle.send(());
    } else {
        // Try to handle cancelling pending tasks, not completely foolproof
        // if the task is still in the task channel however.
        if let Some((idx, _)) = pending.iter().enumerate().find(|(_idx, task)| {
            task.workflow_run_id == workflow_run_id && task.attempt == attempt && task.loc == loc
        }) {
            pending.remove(idx);
            send_cancelled(event_sender, workflow_run_id, attempt, loc).await?;
        }
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
        let stderr = background_task.stderr;
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
    running_cancellations: &mut RunningCancellations,
    running: &mut RunningFutures<'_>,
    worker_commands: &WorkerCommands,
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
            worker_commands,
            &internal_task.worker_name,
            worker_args_path,
            &internal_task.environment,
            internal_task.logs.is_some(),
        )
    };
    let child = match res {
        Ok(child) => child,
        Err(err) => {
            tracing::debug!(workflow_run_id = %workflow_run_id, attempt, loc = %loc, "Failed to spawn worker: {err}");
            send_error(event_sender, workflow_run_id, attempt, loc, &err).await?;
            return Ok(());
        }
    };
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
    let (cancel, cancel_request) = oneshot::channel();
    let task = tokio::spawn(
        wait_for_worker(child, internal_task.logs, cancel_request).instrument(parent_span),
    );

    running_cancellations.insert((workflow_run_id, attempt, loc), cancel);
    let task = task.map(move |result| {
        let (status, stderr) = match result {
            Ok((status, stderr)) => (
                status.into_diagnostic().wrap_err("Failed to run worker"),
                stderr,
            ),
            Err(err) => (Err(miette!("Failed to join task: {err}")), None),
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
            status,
        )
    });

    running.push(task.boxed());

    Ok(())
}

const MAX_PENDING_TASKS: usize = 64;

async fn process_tasks(
    mut task_receiver: TaskReceiver,
    mut cancel_receiver: CancelReceiver,
    mut event_sender: EventSender,
    asset_storage_registry: AssetStorageRegistry,
    worker_commands: WorkerCommands,
    mut running_cancellations: RunningCancellations,
    max_concurrency: NonZero<usize>,
) {
    let max_concurrency: usize = max_concurrency.into();
    let mut running: RunningFutures = FuturesUnordered::new();
    let mut pending: VecDeque<BackgroundTaskPlan> = VecDeque::with_capacity(MAX_PENDING_TASKS);

    loop {
        tokio::select! {
            biased; // Prefer to deal with futures in the defined order.
            // Conditional guards on branches are always evaluated before evaluating their pattern.

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
                    &mut running_cancellations,
                    &mut pending,
                    workflow_run_id,
                    attempt,
                    loc
                )
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
                running_cancellations.remove(&(task.workflow_run_id, task.attempt, task.loc.clone()));
                let exit_code = match result {
                    Ok(Some(exit_code)) => exit_code,
                    Ok(None) => {
                        send_cancelled(&mut event_sender, task.workflow_run_id, task.attempt, task.loc)
                            .await.expect("Failed to send cancellation event");
                        continue;
                    }
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
                    &asset_storage_registry,
                    task,
                    exit_code,
                )
                .await
                .expect("Failed to complete task");
            }
            // We have space for a new task and there is at least one task pending.
            () = std::future::ready(()), if running.len() < max_concurrency && !pending.is_empty() => {
                let task = pending.pop_front().expect("Pending queue was unexpectedly empty");
                start_task(
                    &mut event_sender,
                    &mut running_cancellations,
                    &mut running,
                    &worker_commands,
                    task,
                )
                .await
                .expect("Failed to start task");
            }
            // A task has been submitted, don't accept it unless we have space for pending tasks.
            Some(internal_task) = task_receiver.next(), if pending.len() < MAX_PENDING_TASKS => {
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
        worker_commands: WorkerCommands,
    ) -> miette::Result<Self> {
        if let Some(subprocess_storage) = asset_storage_registry.get(subprocess_storage_name) {
            let subprocess_storage: &dyn Any = subprocess_storage as &dyn Any;
            if subprocess_storage.is::<FileAssetStorage>() {
                return Err(miette!(
                    "subprocess_storage_name must be an instance of FileAssetStorage"
                ));
            }
        } else {
            return Err(miette!("subprocess_storage_name not in registry"));
        }

        if !asset_storage_registry.contains_key(output_storage_name) {
            return Err(miette!("output_storage_name not in registry"));
        }
        validate_worker_commands(&worker_commands)?;

        let background_asset_storage_registry = Arc::clone(asset_storage_registry);
        let (task_sender, task_receiver) = mpsc::channel(64);
        let (event_sender, event_receiver) = mpsc::channel(64);
        let (cancel_sender, cancel_receiver) = mpsc::channel(64);
        let running_cancellations = HashMap::new();
        let background_task = tokio::spawn(process_tasks(
            task_receiver,
            cancel_receiver,
            event_sender,
            background_asset_storage_registry,
            worker_commands,
            running_cancellations,
            std::thread::available_parallelism().into_diagnostic()?,
        ));

        let asset_storage_registry = Arc::clone(asset_storage_registry);
        Ok(Self {
            task_sender,
            cancel_sender,
            event_receiver: Mutex::new(Some(event_receiver)),
            background_abort_handle: background_task.abort_handle(),
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

fn spawn_worker(
    commands: &WorkerCommands,
    worker_name: &str,
    worker_args_path: &Path,
    environment: &HashMap<String, String>,
    capture_stdout: bool,
) -> miette::Result<tokio::process::Child> {
    let (cmd, args) = worker_invocation(commands, worker_name, worker_args_path)?;
    let mut command = Command::new(&cmd);

    command.envs(environment);
    let cx = tracing::Span::current().context();
    inject_trace_context(&cx, &mut CommandEnvCarrier(&mut command));

    command
        .args(args)
        .kill_on_drop(true)
        .stdout(if capture_stdout {
            Stdio::piped()
        } else {
            Stdio::null()
        })
        .stderr(Stdio::piped());

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

async fn wait_for_worker(
    mut child: tokio::process::Child,
    logs: Option<TaskLogWriter>,
    cancel_request: oneshot::Receiver<()>,
) -> (std::io::Result<Option<ExitStatus>>, Option<String>) {
    let stdout = child
        .stdout
        .take()
        .map(|pipe| tokio::spawn(capture_output(pipe, logs.clone(), LogSource::Stdout)));
    let stderr = child
        .stderr
        .take()
        .map(|pipe| tokio::spawn(capture_output(pipe, logs, LogSource::Stderr)));
    let result = tokio::select! {
        status = child.wait() => status.map(Some),
        Ok(()) = cancel_request => child.kill().await.map(|()| None),
    };
    let (_, stderr) = tokio::join!(finish_capture(stdout), finish_capture(stderr));
    (result, stderr)
}

async fn finish_capture(reader: Option<JoinHandle<String>>) -> Option<String> {
    let mut reader = reader?;
    // Descendants can inherit the pipe even after the direct worker exits.
    if let Ok(result) = tokio::time::timeout(Duration::from_secs(2), &mut reader).await {
        result.ok()
    } else {
        reader.abort();
        tracing::warn!("Worker output pipe remained open after exit");
        None
    }
}

async fn capture_output(
    mut pipe: impl AsyncRead + Unpin,
    logs: Option<TaskLogWriter>,
    source: LogSource,
) -> String {
    const STDERR_TAIL_BYTES: usize = 16 * 1024;
    let mut buffer = [0; CHUNK_BYTES];
    let mut pending = 0;
    let mut tail = Vec::new();
    loop {
        let read = match pipe.read(&mut buffer[pending..]).await {
            Ok(read) => read,
            Err(err) => {
                tracing::warn!(%source, "Failed to read worker output: {err}");
                break;
            }
        };
        let len = pending + read;
        let end = if read == 0 {
            len
        } else {
            complete_utf8_prefix(&buffer[..len])
        };
        let chunk = &buffer[..end];
        if source == LogSource::Stderr {
            let discard = (tail.len() + chunk.len()).saturating_sub(STDERR_TAIL_BYTES);
            tail.drain(..discard);
            tail.extend_from_slice(chunk);
        }
        if let Some(logs) = &logs {
            logs.write(source, &String::from_utf8_lossy(chunk)).await;
        }
        if read == 0 {
            break;
        }
        buffer.copy_within(end..len, 0);
        pending = len - end;
    }
    String::from_utf8_lossy(&tail).into_owned()
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
        self.execute_with_logs(task_plans, None)
    }

    fn execute_with_logs(
        &self,
        task_plans: Vec<TaskPlan>,
        logs: Option<TaskLogSink>,
    ) -> BoxFuture<'_, miette::Result<()>> {
        let fut = async move {
            let mut task_sender = self.task_sender.clone();

            for task_plan in task_plans {
                let task_logs = logs.as_ref().map(|logs| logs.for_task(&task_plan));
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
                        logs: task_logs,
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
        let executor =
            SubprocessExecutor::try_new(&registry, "file", "file", WorkerCommands::new())?;

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
        let executor = SubprocessExecutor::try_new(
            &registry,
            "file",
            output_storage_name,
            WorkerCommands::new(),
        )?;

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
        let executor = SubprocessExecutor::try_new(
            &registry,
            "file",
            output_storage_name,
            WorkerCommands::new(),
        )?;

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
        let executor = SubprocessExecutor::try_new(
            &registry,
            "file",
            output_storage_name,
            WorkerCommands::new(),
        )?;

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
        let executor =
            SubprocessExecutor::try_new(&registry, "file", "file", WorkerCommands::new())?;

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
        let executor =
            SubprocessExecutor::try_new(&registry, "file", "file", WorkerCommands::new())?;

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
        let executor =
            SubprocessExecutor::try_new(&registry, "file", "file", WorkerCommands::new())?;

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
        let executor =
            SubprocessExecutor::try_new(&registry, "file", "file", WorkerCommands::new())?;

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
        let executor =
            SubprocessExecutor::try_new(&registry, "file", "file", WorkerCommands::new())?;

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

    #[cfg(unix)]
    #[rstest::rstest]
    #[case::complete(Some(0))]
    #[case::fail(Some(1))]
    #[case::cancel(None)]
    #[tokio::test]
    async fn subprocess_captures_output_before_exit(
        #[case] exit: Option<i32>,
    ) -> miette::Result<()> {
        use crate::{
            state::{InMemoryRuntimeState, RuntimeState},
            worker_logs::{WorkerLogQuery, test_log_sink},
        };
        let state: Arc<dyn RuntimeState> = Arc::new(InMemoryRuntimeState::new());
        let (run, sink) = test_log_sink(Arc::clone(&state), "process").await?;
        let (registry, _, dir) = test_storage_registry(vec![], vec![]).await;
        let gate = dir.path().join("release");
        let shell = concat!(
            "printf 'live stdout\n'; printf 'live stderr\n' >&2; ",
            "printf '\\377' >&2; ",
            "while [ ! -f \"$RELEASE\" ]; do sleep 0.01; done; ",
            "printf 'final stdout'; printf 'final stderr' >&2; exit \"$EXIT_CODE\"",
        );
        let executor = SubprocessExecutor::try_new(
            &registry,
            "file",
            "memory",
            HashMap::from([(
                "log-worker".into(),
                vec!["sh".into(), "-c".into(), shell.into(), "log-worker".into()],
            )]),
        )?;
        let loc = Location::new("N0.M1.N2")?;
        let task = TaskPlan {
            workflow_run_id: run.run_id(),
            loc: loc.clone(),
            worker_name: "log-worker".into(),
            environment: HashMap::from([
                ("RELEASE".into(), json!(gate)),
                (
                    "EXIT_CODE".into(),
                    json!(exit.unwrap_or_default().to_string()),
                ),
            ]),
            ..Default::default()
        };
        let events = executor.listen()?;
        executor.execute_with_logs(vec![task], Some(sink)).await?;

        let read = || {
            state.read_worker_logs(WorkerLogQuery {
                run_id: run.run_id(),
                attempt: 0,
                location: Some(loc.clone()),
                after: 0,
                source: None,
                limit: 100,
            })
        };
        tokio::time::timeout(Duration::from_secs(5), async {
            loop {
                let records = read().await?.unwrap();
                if [LogSource::Stdout, LogSource::Stderr].iter().all(|source| {
                    records
                        .iter()
                        .any(|record| record.source == *source && record.message.contains("live"))
                }) {
                    return Ok::<_, miette::Report>(());
                }
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .into_diagnostic()??;

        if exit.is_some() {
            tokio::fs::write(gate, []).await.into_diagnostic()?;
        } else {
            executor.cancel(run.run_id(), 0, vec![loc.clone()]).await?;
        }
        let terminal =
            tokio::time::timeout(Duration::from_secs(5), events.take(4).collect::<Vec<_>>())
                .await
                .into_diagnostic()?;
        let RuntimeEvent::WorkflowRun {
            event: WorkflowRunEvent::NodeEvents(nodes),
            ..
        } = &terminal[3]
        else {
            panic!("Expected a terminal task event");
        };
        match (exit, &nodes[0].status) {
            (None, NodeStatus::Cancelled) | (Some(0), NodeStatus::Complete { .. }) => {}
            (
                Some(1),
                NodeStatus::Error {
                    detail: Some(detail),
                    ..
                },
            ) => assert!(detail.contains("final stderr")),
            unexpected => panic!("Unexpected terminal state: {unexpected:?}"),
        }
        let records = read().await?.unwrap();
        let stderr = records
            .iter()
            .filter(|record| record.source == LogSource::Stderr)
            .map(|record| record.message.as_str())
            .collect::<String>();
        assert!(stderr.contains('�'));
        assert_eq!(stderr.contains("final stderr"), exit.is_some());
        Ok(())
    }

    #[tokio::test]
    async fn execute_subprocess_with_worker_command_override() -> miette::Result<()> {
        let (registry, input_sets, _dir) = test_storage_registry(
            vec![json!({"greeting": "hello ", "subject": "override"})],
            vec![],
        )
        .await;
        let executor = SubprocessExecutor::try_new(
            &registry,
            "file",
            "memory",
            WorkerCommands::from([(
                "hello-world-worker".to_string(),
                vec!["env".to_string(), "tkr-hello-world-worker".to_string()],
            )]),
        )?;
        let stream = executor.listen()?;
        executor
            .execute(vec![TaskPlan {
                worker_name: "hello-world-worker".to_string(),
                task_name: "greet".to_string(),
                inputs: input_sets[0].clone(),
                outputs: HashSet::from(["value".to_string()]),
                ..Default::default()
            }])
            .await?;

        let events = stream.take(4).collect::<Vec<_>>().await;
        assert_registry_contains_values(
            &registry,
            "memory",
            &events[3].clone().outputs()[0],
            json!({"value": "hello override"}),
        )
        .await;
        Ok(())
    }
}
