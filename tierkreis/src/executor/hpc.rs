/*!
This module defines the [`HPCExecutor`] struct which implements [Executor]
by running subprocesses.
*/
use std::{
    collections::{HashMap, HashSet},
    path::PathBuf,
    process::ExitStatus,
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
use serde::{Deserialize, Serialize};
use tokio::{
    io::AsyncReadExt,
    task::{AbortHandle, JoinHandle},
};
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
    executor::{
        interface::{Executor, TaskPlan, WorkerSpec},
        slurm::{parse_and_extract_job_id, poll_slurm_status, submit_job, write_jobscript},
    },
    location::Location,
};

/// [`HPCResourceSpec`] determines what Resources should be available to the
/// [`HPCExecutor`] or what is requested as part of a [`TaskPlan`].
#[derive(Clone, Debug, PartialEq, Default)]
pub struct HPCResourceSpec {
    nodes: u32,
    cores_per_node: u32,
    memory_per_node_gb: u32,
    gpus_per_node: u32,
}

impl HPCResourceSpec {
    pub fn new(
        nodes: u32,
        cores_per_node: u32,
        memory_per_node_gb: u32,
        gpus_per_node: u32,
    ) -> Self {
        Self {
            nodes,
            cores_per_node,
            memory_per_node_gb,
            gpus_per_node,
        }
    }

    pub fn satisfies(&self, other: &HPCResourceSpec) -> bool {
        self.nodes >= other.nodes
            && self.cores_per_node >= other.cores_per_node
            && self.memory_per_node_gb >= other.memory_per_node_gb
            && self.gpus_per_node >= other.gpus_per_node
    }
}

/// [`HPCEnvironmentSpec`] determines the default execution environment of
/// [`HPCExecutor`] or what is requested as part of a [`TaskPlan`].
#[derive(Clone, Debug, PartialEq, Default)]
pub struct HPCEnvironmentSpec {
    mpi_available: bool,
}
impl HPCEnvironmentSpec {
    pub fn new(mpi_available: bool) -> Self {
        Self { mpi_available }
    }

    pub fn satisfies(&self, other: &HPCEnvironmentSpec) -> bool {
        !other.mpi_available || self.mpi_available
    }
}

#[derive(Clone)]
pub struct BackgroundTaskPlan {
    workflow_run_id: Uuid,
    attempt: u32,
    loc: Location,
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
    workflow_run_id: Uuid,
    attempt: u32,
    loc: Location,
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
type CancelSender = mpsc::Sender<(Uuid, u32, Location)>;
type CancelReceiver = mpsc::Receiver<(Uuid, u32, Location)>;
type RunningFutures = FuturesUnordered<JoinHandle<BackgroundTask>>;
type AbortHandles = HashMap<(Uuid, u32, Location), AbortHandle>;

/// Identical to subprocess executor
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

/// Identical to subprocess executor
async fn process_finished_task(
    event_sender: &mut EventSender,
    abort_handles: &mut AbortHandles,
    asset_storage_registry: &AssetStorageRegistry,
    background_task: BackgroundTask,
) -> miette::Result<()> {
    let loc = background_task.loc;
    let outputs = background_task.outputs;
    let output_storage_name = background_task.output_storage_name;
    let workflow_run_id = background_task.workflow_run_id;
    let attempt = background_task.attempt;
    let exit_status = background_task.exit_status;

    abort_handles.remove(&(workflow_run_id, attempt, loc.clone()));
    match exit_status {
        Ok(status) => {
            if status.success() {
                let outputs =
                    transfer_assets(asset_storage_registry, &output_storage_name, &outputs).await;
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
                                error: format!("Subprocess failed with error code: {status}"),
                                detail: stderr,
                            },
                        }),
                    })
                    .await
                    .map_err(|err| miette!("Failed to send error event: {err}"))?;
            }
        }
        Err(err) => {
            send_error(
                event_sender,
                workflow_run_id,
                attempt,
                loc,
                &miette!("Failed to run worker: {err}"),
            )
            .await?;
        }
    }

    Ok(())
}

/// Similar to subrocess executor
/// Writes a jpobscript and then uses it to submit slurm
/// Instead of waiting for the subprocess, polls slurm for job status
async fn start_task(
    event_sender: &mut EventSender,
    abort_handles: &mut AbortHandles,
    running: &mut RunningFutures,
    internal_task: BackgroundTaskPlan,
) -> miette::Result<()> {
    let task = internal_task.clone();
    let loc = internal_task.loc;
    let workflow_run_id = internal_task.workflow_run_id;
    let attempt = internal_task.attempt;
    send_running(event_sender, workflow_run_id, attempt, loc.clone()).await?;

    write_jobscript(&task, &internal_task.script_path)?;
    // TODO: this is different than before, does it matter?
    let worker_args = internal_task.worker_args;
    let done_file = internal_task.done_file;
    let res = submit_job(&internal_task.script_path);
    let mut child = match res {
        Ok(child) => child,
        Err(err) => {
            send_error(event_sender, workflow_run_id, attempt, loc, &err).await?;
            return Ok(());
        }
    };
    let stderr = read_stderr(&mut child);
    let job_id = parse_and_extract_job_id(&mut child).await.unwrap().unwrap();

    let outputs = internal_task.outputs;
    let output_storage_name = internal_task.output_storage_name;
    let background_loc = loc.clone();
    dbg!(&job_id);
    let task = tokio::task::spawn(async move {
        let exit_status =
            poll_slurm_status(&job_id, Duration::new(1, 0), Duration::new(60, 0)).await;
        BackgroundTask {
            workflow_run_id,
            attempt,
            loc: background_loc,
            hpc_id: job_id,
            output_storage_name,
            exit_status,
            outputs,
            stderr,
            _worker_args: worker_args,
            _done_file: done_file,
        }
    });

    abort_handles.insert((workflow_run_id, attempt, loc), task.abort_handle());
    running.push(task);

    Ok(())
}

/// Identical to subprocess executor
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
                process_cancelled_task(&mut event_sender, &mut abort_handles, workflow_run_id, attempt, loc)
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
            else => break
        }
    }
}

/// [`SubprocessExecutor`] defines an [Executor] that performs Task Nodes using Worker subprocesses.
pub struct HPCExecutor {
    task_sender: TaskSender,
    cancel_sender: CancelSender,
    event_receiver: Mutex<Option<EventReceiver>>,

    // The name of the storage that the subprocess will read
    // and write files from. Must be file based.
    hpc_storage_name: String,
    // The name of the storage to copy outputs to.
    //
    // If this is the same as the subprocess storage name,
    // no copying will occur.
    output_storage_name: String,
    asset_storage_registry: AssetStorageRegistry,
    pub max_resources: HPCResourceSpec,
    pub environment: HPCEnvironmentSpec,
}

type OutputSpecs = (HashMap<String, AssetSpec>, HashMap<String, PathBuf>);

impl HPCExecutor {
    /// Try to create a new [`HPCExecutor`] with an [`AssetStorageRegistry`], a
    /// configured name for an [`AssetStorage`][crate::asset_storage::AssetStorage]
    /// of [`AssetKind::File`][crate::asset_storage::AssetKind::File] where files are written
    /// to for the subprocesses to consume and a configured name for an
    /// [`AssetStorage`][crate::asset_storage::AssetStorage] in the registry that
    /// determines where Assets are saved by default.
    /// Identical to [`SubprocessExecutor::try_new`].
    ///
    /// # Errors
    ///
    /// This function will return Err if the specified `subprocess_storage_name` or
    /// `output_storage_name` does not exist inside the [`AssetStorageRegistry`].
    pub async fn try_new(
        asset_storage_registry: &AssetStorageRegistry,
        hpc_storage_name: &str,
        output_storage_name: &str,
        max_resources: HPCResourceSpec,
        environment: HPCEnvironmentSpec,
    ) -> miette::Result<Self> {
        let asset_storage_registry_lock = asset_storage_registry.read().await;
        if let Some(subprocess_storage) = asset_storage_registry_lock.get(hpc_storage_name) {
            if !matches!(subprocess_storage.kind(), AssetKind::File { .. }) {
                return Err(miette!("hpc_storage_name must be of AssetKind::File"));
            }
        } else {
            return Err(miette!("hpc_storage_name not in registry"));
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
            hpc_storage_name: hpc_storage_name.to_string(),
            output_storage_name: output_storage_name.to_string(),
            asset_storage_registry,

            max_resources,
            environment,
        })
    }

    /// TODO: Query available workers. Identical to subprocess executor
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

    /// Identical except for relative path. TODO: Might not be necessary if we have storage config
    async fn build_inputs(
        &self,
        inputs: &HashMap<String, AssetSpec>,
        base: &std::path::Path,
    ) -> Result<HashMap<String, PathBuf>, miette::Error> {
        let inputs =
            transfer_assets(&self.asset_storage_registry, &self.hpc_storage_name, inputs).await?;
        let inputs = write_input_paths(&inputs)
            .wrap_err("Failed to collect Worker input filepaths")?
            .iter()
            .map(|(k, v)| {
                let rel_path = v.strip_prefix(base).into_diagnostic()?;
                Ok((k.clone(), rel_path.to_path_buf()))
            })
            .collect::<Result<HashMap<_, _>, miette::Error>>()?;
        Ok(inputs)
    }

    /// Identical except for relative path. TODO: Might not be necessary if we have storage config
    async fn build_outputs(
        &self,
        outputs: HashSet<String>,
        base: &std::path::Path,
    ) -> Result<OutputSpecs, miette::Error> {
        let output_specs = reserve_asset_specs(
            &self.asset_storage_registry,
            &self.hpc_storage_name,
            outputs.len(),
        )
        .await?;
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
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn workers(&self) -> BoxFuture<'_, miette::Result<Vec<WorkerSpec>>> {
        self.workers().boxed()
    }

    /// slightly different as it doesnt create a temp file
    fn execute(&self, task_plans: Vec<TaskPlan>) -> BoxFuture<'_, miette::Result<()>> {
        let fut = async {
            //let mut ids = Vec::new();
            let mut task_sender = self.task_sender.clone();

            let base_path = self
                .asset_storage_registry
                .read()
                .await
                .get(&self.hpc_storage_name)
                .ok_or_else(|| miette!("subprocess_storage_name not in registry"))
                .and_then(|file_storage| match file_storage.kind() {
                    AssetKind::File { root } => Ok(root),
                    _ => Err(miette!(
                        "subprocess_storage_name must be of AssetKind::File"
                    )),
                })?;

            for task_plan in task_plans {
                let inputs = self
                    .build_inputs(&task_plan.inputs, &base_path.as_path())
                    .await?;
                let (outputs, output_paths) = self
                    .build_outputs(task_plan.outputs, &base_path.as_path())
                    .await?;

                let tmp_assets =
                    &reserve_asset_specs(&self.asset_storage_registry, &self.hpc_storage_name, 2)
                        .await?;
                let worker_args = tmp_assets[0].path()?;
                let file = std::fs::File::create(&worker_args).into_diagnostic()?;
                let script_path = tmp_assets[1].path()?;
                // Redirect the done_file to a temporary file as we
                // do not need it to figure out if a process has
                // completed currently.
                let done_file = std::path::Path::new("_done").to_path_buf();

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
                        workflow_run_id: task_plan.workflow_run_id,
                        attempt: task_plan.attempt,
                        loc: task_plan.loc,
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

            receiver.take().ok_or(miette!(
                "Failed to listen: Executor is already being listened to."
            ))?
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

    use futures::StreamExt;
    use serde_json::json;

    use super::*;
    use crate::{
        asset_storage::{FileAssetStorage, assert_registry_contains_values, test_storage_registry},
        executor::HPCExecutor,
    };
    // Test that we can launch a task and listen for
    // errors when they occur
    #[tokio::test]
    async fn execute_hpc() -> miette::Result<()> {
        // TODO: overwrite test_storage_registry in a way that the file system is the checkpoints dir
        let file_storage = FileAssetStorage::new(std::path::Path::new(
            "/Users/philipp.seitz/.tierkreis/checkpoints/00000000-0000-0000-0000-000000000016/",
        ));
        let (registry, input_sets, _dir) =
            test_storage_registry(vec![json!({"value": "Test"})], vec![]).await;
        registry
            .write()
            .await
            .insert("checkpoints".to_string(), Box::new(file_storage));
        let mut outputs = HashSet::new();
        outputs.insert("value".to_string());
        let task_plans = vec![TaskPlan {
            loc: Location::default(),
            worker_name: "hello-world-worker".to_string(),
            task_name: "mpi_rank_info_with_input".to_string(),
            outputs,
            inputs: input_sets[0].clone(),

            ..Default::default()
        }];
        let resources = HPCResourceSpec {
            nodes: 2,
            cores_per_node: 1,
            memory_per_node_gb: 1,
            gpus_per_node: 0,
        };
        let environment = HPCEnvironmentSpec {
            mpi_available: true,
        };
        let executor = HPCExecutor::try_new(
            &registry,
            "checkpoints",
            "checkpoints",
            resources,
            environment,
        )
        .await?;

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
        assert_registry_contains_values(
            &registry,
            "checkpoints",
            &events[1].clone().outputs()[0],
            json!({"value": "Rank 0 out of 2 on c1 with value Test.\nRank 1 out of 2 on c2 with value Test."}),
        ).await;

        Ok(())
    }
}
