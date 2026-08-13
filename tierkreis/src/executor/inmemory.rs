/*!
This module defines the [`InMemoryExecutor`] struct which implements [Executor]
by running small tasks from a small work queue.
*/
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    thread::sleep,
    time::Duration,
};

use futures::{
    FutureExt, SinkExt, StreamExt,
    channel::mpsc,
    future::BoxFuture,
    stream::{BoxStream, FuturesUnordered},
};
use miette::{IntoDiagnostic, miette};
use num_complex::Complex64;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::task::{AbortHandle, JoinHandle};
use uuid::Uuid;

use crate::{
    asset_storage::{AssetStorageRegistry, load_assets, save_assets},
    event::{
        EventReceiver, EventSender, NodeStatus, RuntimeEvent, send_cancelled, send_complete,
        send_error, send_running,
    },
    executor::interface::{Executor, TaskPlan, WorkerSpec},
    location::Location,
};

/// [`InMemoryResourceSpec`] determines what Resources should be available to the
/// [`InMemoryExecutor`] or what is requested as part of a [`TaskPlan`].
#[derive(Clone, Debug, PartialEq, Default)]
pub struct InMemoryResourceSpec {}

/// [`InMemoryEnvironmentSpec`] determines the default execution environment of
/// [`InMemoryExecutor`] or what is requested as part of a [`TaskPlan`].
// TODO: It's perhaps a bit unclear what this means for in-memory execution?
#[derive(Clone, Debug, PartialEq, Default)]
pub struct InMemoryEnvironmentSpec {}

struct BackgroundTaskPlan {
    task_plan: TaskPlan,
    output_storage_name: String,
    /// Captured from the caller so in-memory tasks inherit the parent trace context.
    parent_span: tracing::Span,
}

#[derive(Debug)]
struct BackgroundTask {
    workflow_run_id: Uuid,
    attempt: u32,
    loc: Location,
    output_storage_name: String,
    task_output: miette::Result<HashMap<String, Vec<u8>>>,
}

type TaskSender = mpsc::Sender<BackgroundTaskPlan>;
type TaskReceiver = mpsc::Receiver<BackgroundTaskPlan>;
type CancelSender = mpsc::Sender<(Uuid, u32, Location)>;
type CancelReceiver = mpsc::Receiver<(Uuid, u32, Location)>;

fn extract_value<'a, T>(inputs: &'a HashMap<String, Vec<u8>>, name: &str) -> miette::Result<T>
where
    T: Deserialize<'a>,
{
    let asset = inputs
        .get(name)
        .ok_or_else(|| miette!("Missing input: {name}"))?;
    let val = serde_json::from_slice(asset).into_diagnostic()?;

    Ok(val)
}

fn extract_optional_value<'a, T>(
    inputs: &'a HashMap<String, Vec<u8>>,
    name: &str,
) -> miette::Result<Option<T>>
where
    T: Deserialize<'a>,
{
    let val = inputs
        .get(name)
        .map(|asset| {
            let val: Option<T> = serde_json::from_slice(asset).into_diagnostic()?;
            Ok::<_, miette::Report>(val)
        })
        .transpose()?
        .flatten();

    Ok(val)
}

fn output_value(
    outputs: &mut HashMap<String, Vec<u8>>,
    value: impl Serialize,
) -> miette::Result<()> {
    outputs.insert(
        "value".to_string(),
        serde_json::to_vec(&value).into_diagnostic()?,
    );
    Ok(())
}

macro_rules! un_op {
    ($inputs:expr, $outputs:expr, $op:tt, $input_type:ty) => {{
        let a: $input_type = extract_value($inputs, "a")?;

        output_value(&mut $outputs, $op a)?;
    }};
}

macro_rules! bin_op {
    ($inputs:expr, $outputs:expr, $op:tt, $input_type:ty) => {{
        let a: $input_type = extract_value($inputs, "a")?;
        let b: $input_type = extract_value($inputs, "b")?;

        output_value(&mut $outputs, a $op b)?;
    }};
}

macro_rules! un_method {
    ($inputs:expr, $outputs:expr, $op:tt, $input_type:ty) => {{
        let a: $input_type = extract_value($inputs, "a")?;

        output_value(&mut $outputs, a.$op())?;
    }};
}

macro_rules! bin_method {
    ($inputs:expr, $outputs:expr, $op:tt, $input_type:ty) => {{
        let a: $input_type = extract_value($inputs, "a")?;
        let b: $input_type = extract_value($inputs, "b")?;

        output_value(&mut $outputs, a.$op(b))?;
    }};
}

fn run_builtin(
    task_name: &str,
    inputs: &HashMap<String, Vec<u8>>,
) -> miette::Result<HashMap<String, Vec<u8>>> {
    let mut outputs = HashMap::new();
    match task_name {
        // i64 operations
        "ieq" => bin_op!(inputs, outputs, ==, i64),
        "ine" => bin_op!(inputs, outputs, !=, i64),
        "ilt_s" => bin_op!(inputs, outputs, <, i64),
        // igt is the legacy name
        "igt_s" | "igt" => bin_op!(inputs, outputs, >, i64),
        "ilte_s" => bin_op!(inputs, outputs, <=, i64),
        "igte_s" => bin_op!(inputs, outputs, >=, i64),
        "imax_s" => bin_method!(inputs, outputs, max, i64),
        "imin_s" => bin_method!(inputs, outputs, min, i64),
        "iadd" => bin_op!(inputs, outputs, +, i64),
        "isub" => bin_op!(inputs, outputs, -, i64),
        "ineg" => un_op!(inputs, outputs, -, i64),
        // itimes is the legacy name
        "imul" | "itimes" => bin_op!(inputs, outputs, *, i64),
        // idivide is the legacy name
        "idiv_s" | "idivide" => bin_op!(inputs, outputs, /, i64),
        // mod is the legacy name
        "imod_s" | "mod" => bin_op!(inputs, outputs, %, i64),
        "iabs" => un_method!(inputs, outputs, abs, i64),
        "iand" => bin_op!(inputs, outputs, &, i64),
        "ior" => bin_op!(inputs, outputs, |, i64),
        "ixor" => bin_op!(inputs, outputs, ^, i64),
        "itostring_s" => un_method!(inputs, outputs, to_string, i64),

        // f64 operations
        #[allow(clippy::float_cmp)]
        "feq" => bin_op!(inputs, outputs, ==, f64),
        #[allow(clippy::float_cmp)]
        "fne" => bin_op!(inputs, outputs, !=, f64),
        "flt" => bin_op!(inputs, outputs, <, f64),
        // igt is the legacy name
        "fgt" => bin_op!(inputs, outputs, >, f64),
        "flte" => bin_op!(inputs, outputs, <=, f64),
        "fgte" => bin_op!(inputs, outputs, >=, f64),
        "fmax" => bin_method!(inputs, outputs, max, f64),
        "fmin" => bin_method!(inputs, outputs, min, f64),
        "fadd" => bin_op!(inputs, outputs, +, f64),
        "fsub" => bin_op!(inputs, outputs, -, f64),
        "fneg" => un_op!(inputs, outputs, -, f64),
        "fabs" => un_method!(inputs, outputs, abs, f64),
        "fmul" => bin_op!(inputs, outputs, *, f64),
        "idiv" => bin_op!(inputs, outputs, /, f64),
        "ffloor" => un_method!(inputs, outputs, floor, f64),
        "fceil" => un_method!(inputs, outputs, ceil, f64),
        "ftostring" => un_method!(inputs, outputs, to_string, f64),

        // "Value" operations.
        "eq" => bin_op!(inputs, outputs, ==, Value),
        "ne" | "neq" => bin_op!(inputs, outputs, !=, Value),

        // Legacy built-in name.
        "str" | "tkr_str" => {
            let value: Value = extract_value(inputs, "value")?;

            output_value(&mut outputs, value.to_string())?;
        }
        // Legacy built-in name.
        "tuple" | "tkr_tuple" => {
            let a: Value = extract_value(inputs, "a")?;
            let b: Value = extract_value(inputs, "b")?;

            output_value(&mut outputs, [a, b])?;
        }
        "untuple" => {
            let value: (Value, Value) = extract_value(inputs, "value")?;

            outputs.insert(
                "a".to_string(),
                serde_json::to_vec(&value.0).into_diagnostic()?,
            );
            outputs.insert(
                "b".to_string(),
                serde_json::to_vec(&value.1).into_diagnostic()?,
            );
        }
        "range" | "tkr_range" => {
            let start: i64 = extract_value(inputs, "start")?;
            let stop: i64 = extract_value(inputs, "stop")?;
            let maybe_step: Option<i64> = extract_optional_value(inputs, "step")?;

            let range = start..stop;

            let out = if let Some(step) = maybe_step {
                range
                    .step_by(step.try_into().into_diagnostic()?)
                    .collect::<Vec<_>>()
            } else {
                range.collect::<Vec<_>>()
            };

            output_value(&mut outputs, out)?;
        }
        "conjugate" => {
            let z: Complex64 = extract_value(inputs, "z")?;

            output_value(&mut outputs, z.conj())?;
        }
        "sleep" => {
            let delay_seconds: f64 = extract_value(inputs, "delay_seconds")?;
            sleep(Duration::from_secs_f64(delay_seconds));

            output_value(&mut outputs, true)?;
        }
        task => return Err(miette!("Unknown task: `{task}`")),
    }
    Ok(outputs)
}

type RunningFutures = FuturesUnordered<JoinHandle<BackgroundTask>>;
type AbortHandles = Arc<Mutex<HashMap<(Uuid, u32, Location), AbortHandle>>>;

async fn process_cancelled_task(
    event_sender: &mut EventSender,
    abort_handles: &AbortHandles,
    workflow_run_id: Uuid,
    attempt: u32,
    loc: Location,
) -> miette::Result<()> {
    let handle = abort_handles
        .lock()
        .unwrap()
        .remove(&(workflow_run_id, attempt, loc.clone()));
    if let Some(handle) = handle {
        handle.abort();
        send_cancelled(event_sender, workflow_run_id, attempt, loc).await?;
    }
    Ok(())
}

async fn process_finished_task(
    event_sender: &mut EventSender,
    abort_handles: &AbortHandles,
    asset_storage_registry: &AssetStorageRegistry,
    background_task: BackgroundTask,
) -> miette::Result<()> {
    let loc = background_task.loc;
    let task_outputs = background_task.task_output;
    let output_storage_name = background_task.output_storage_name;
    let workflow_run_id = background_task.workflow_run_id;
    let attempt = background_task.attempt;

    abort_handles
        .lock()
        .unwrap()
        .remove(&(workflow_run_id, attempt, loc.clone()));

    let outputs = match task_outputs {
        Ok(outputs) => outputs,
        Err(err) => {
            send_error(event_sender, workflow_run_id, attempt, loc, &err).await?;
            return Ok(());
        }
    };

    let outputs = save_assets(asset_storage_registry, &output_storage_name, outputs).await;
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
        Err(err) => send_error(event_sender, workflow_run_id, attempt, loc, &err).await?,
    }

    Ok(())
}

async fn start_task(
    event_sender: &mut EventSender,
    abort_handles: &AbortHandles,
    asset_storage_registry: &AssetStorageRegistry,
    running: &mut RunningFutures,
    internal_task: BackgroundTaskPlan,
) -> miette::Result<()> {
    let task_plan = internal_task.task_plan;
    let loc = task_plan.loc;
    let output_storage_name = internal_task.output_storage_name;
    let workflow_run_id = task_plan.workflow_run_id;
    let attempt = task_plan.attempt;
    send_running(event_sender, workflow_run_id, attempt, loc.clone()).await?;

    let res = load_assets(asset_storage_registry, &task_plan.inputs).await;

    let inputs = match res {
        Ok(inputs) => inputs,
        Err(err) => {
            send_error(event_sender, workflow_run_id, attempt, loc, &err).await?;
            return Ok(());
        }
    };

    let task_loc = loc.clone();

    let task_name = task_plan.task_name;
    let parent_span = internal_task.parent_span;
    let task = tokio::task::spawn_blocking(move || {
        parent_span.in_scope(|| {
            let span = tracing::info_span!(
                "run_builtin",
                run_id = %workflow_run_id,
                attempt,
                loc = %task_loc,
                task_name = %task_name,
            );
            span.in_scope(|| BackgroundTask {
                workflow_run_id,
                attempt,
                loc: task_loc,
                output_storage_name,
                task_output: run_builtin(&task_name, &inputs),
            })
        })
    });

    abort_handles
        .lock()
        .unwrap()
        .insert((workflow_run_id, attempt, loc.clone()), task.abort_handle());
    running.push(task);

    Ok(())
}

async fn process_tasks(
    mut task_receiver: TaskReceiver,
    mut cancel_receiver: CancelReceiver,
    mut event_sender: EventSender,
    asset_storage_registry: AssetStorageRegistry,
    abort_handles: AbortHandles,
) {
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
                process_cancelled_task(&mut event_sender, &abort_handles, workflow_run_id, attempt, loc)
                    .await
                    .expect("Failed to cancel task");
            }
            // A task has completed
            Some(res) = running.next() => {
                let background_task = match res {
                    Ok(ok) => ok,
                    Err(err) => panic!("Failed to join to future: {err}"),
                };
                tracing::debug!(
                    workflow_run_id = %background_task.workflow_run_id,
                    attempt = %background_task.attempt,
                    loc = %background_task.loc,
                    "Task completed"
                );
                process_finished_task(
                    &mut event_sender,
                    &abort_handles,
                    &asset_storage_registry,
                    background_task,
                )
                .await
                .expect("Failed to complete task");
            }
            // A task has been submitted
            Some(internal_task) = task_receiver.next() => {
                tracing::debug!(
                    workflow_run_id = %internal_task.task_plan.workflow_run_id,
                    attempt = %internal_task.task_plan.attempt,
                    loc = %internal_task.task_plan.loc,
                    "Received task {}",
                    internal_task.task_plan.task_name
                );
                start_task(
                    &mut event_sender,
                    &abort_handles,
                    &asset_storage_registry,
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

/// [`InMemoryExecutor`] defines an [Executor] that performs Task Nodes in the same runtime.
///
/// These Tasks should be short lived Tasks where we want to avoid the overhead of spinning
/// up an entirely new process.
pub struct InMemoryExecutor {
    task_sender: TaskSender,
    cancel_sender: CancelSender,
    event_receiver: Mutex<Option<EventReceiver>>,
    output_storage_name: String,
    abort_handles: AbortHandles,
}

impl InMemoryExecutor {
    /// Try to create a new [`InMemoryExecutor`] with an [`AssetStorageRegistry`] and
    /// a configured name for an [`AssetStorage`][crate::asset_storage::AssetStorage]
    /// in the registry that determines where Assets are saved by default.
    ///
    /// # Errors
    ///
    /// This function will return Err if the specified `output_storage_name` does not exist
    /// inside the [`AssetStorageRegistry`].
    pub async fn try_new(
        asset_storage_registry: &AssetStorageRegistry,
        output_storage_name: &str,
    ) -> miette::Result<Self> {
        let asset_storage_registry_lock = asset_storage_registry.read().await;
        if !asset_storage_registry_lock.contains_key(output_storage_name) {
            return Err(miette!("output_storage_name not in registry"));
        }

        let asset_storage_registry = Arc::clone(asset_storage_registry);

        let (task_sender, task_receiver) = mpsc::channel(64);
        let (event_sender, event_receiver) = mpsc::channel(64);
        let (cancel_sender, cancel_receiver) = mpsc::channel(64);
        let abort_handles = Arc::new(Mutex::new(HashMap::new()));
        tokio::spawn(Box::pin(process_tasks(
            task_receiver,
            cancel_receiver,
            event_sender,
            asset_storage_registry,
            Arc::clone(&abort_handles),
        )));

        Ok(Self {
            task_sender,
            cancel_sender,
            event_receiver: Mutex::new(Some(event_receiver)),
            output_storage_name: output_storage_name.to_string(),
            abort_handles,
        })
    }
}

impl Executor for InMemoryExecutor {
    fn workers(&self) -> BoxFuture<'_, miette::Result<Vec<WorkerSpec>>> {
        futures::future::ok(vec![WorkerSpec {
            worker_name: "builtin".to_string(),
        }])
        .boxed()
    }

    fn execute(&self, task_plans: Vec<TaskPlan>) -> BoxFuture<'_, miette::Result<()>> {
        let fut = async {
            let mut task_sender = self.task_sender.clone();

            for task_plan in task_plans {
                let output_storage_name = task_plan
                    .output_storage_name
                    .clone()
                    .unwrap_or_else(|| self.output_storage_name.clone());

                task_sender
                    .send(BackgroundTaskPlan {
                        task_plan,
                        output_storage_name,
                        parent_span: tracing::Span::current(),
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

            receiver.take().ok_or_else(|| {
                miette!("Failed to listen: Executor is already being listened to.")
            })?
        };
        Ok(channel.boxed())
    }

    fn cancel(
        &self,
        workflow_run_id: Uuid,
        attempt_id: u32,
        task_locations: Vec<Location>,
    ) -> BoxFuture<'_, miette::Result<()>> {
        let mut cancel_sender = self.cancel_sender.clone();
        let fut = async move {
            for task_location in task_locations {
                cancel_sender
                    .send((workflow_run_id, attempt_id, task_location))
                    .await
                    .into_diagnostic()?;
            }
            Ok(())
        };
        fut.boxed()
    }

    fn known_tasks(
        &self,
        tasks: Vec<(Uuid, u32, Location)>,
    ) -> BoxFuture<'_, miette::Result<Vec<(Uuid, u32, Location, NodeStatus)>>> {
        let abort_handles = Arc::clone(&self.abort_handles);
        async move {
            let abort_handles = abort_handles.lock().unwrap();
            Ok(tasks
                .into_iter()
                .map(|(workflow_run_id, attempt, loc)| {
                    let status =
                        if abort_handles.contains_key(&(workflow_run_id, attempt, loc.clone())) {
                            NodeStatus::Running { state_update: None }
                        } else {
                            NodeStatus::Unknown
                        };
                    (workflow_run_id, attempt, loc, status)
                })
                .collect())
        }
        .boxed()
    }
}

#[cfg(test)]
mod tests {
    use futures::StreamExt;
    use rstest::rstest;
    use serde_json::json;

    use crate::{
        asset_storage::{assert_registry_contains_values, test_storage_registry},
        event::{NodeEvent, NodeStatus, WorkflowRunEvent},
        executor::interface::TaskPlan,
    };

    use super::*;

    #[tokio::test]
    async fn inmemory_workers() -> miette::Result<()> {
        let (registry, _, _) = test_storage_registry(vec![], vec![]).await;
        let executor = InMemoryExecutor::try_new(&registry, "memory").await?;

        let workers = executor.workers().await?;

        // We should expect the builtin worker to be available.
        assert!(
            workers
                .iter()
                .any(|workers| workers.worker_name == "builtin")
        );

        Ok(())
    }

    // Test that we can launch a single task and listen for
    // the status changes.
    #[rstest]
    #[case("memory")]
    #[case("file")]
    #[tokio::test]
    async fn execute_inmemory(#[case] default_storage_name: &str) -> miette::Result<()> {
        let (registry, input_sets, _dir) =
            test_storage_registry(vec![json!({"a": 1, "b": 3})], vec![]).await;

        let task_plans = vec![TaskPlan {
            worker_name: "builtin".to_string(),
            task_name: "iadd".to_string(),

            inputs: input_sets[0].clone(),

            ..Default::default()
        }];
        let executor = InMemoryExecutor::try_new(&registry, default_storage_name).await?;

        let stream = executor.listen()?;
        executor.execute(task_plans).await?;

        let events = stream.take(2).collect::<Vec<_>>().await;
        dbg!(&events);
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
            default_storage_name,
            &events[1].clone().outputs()[0],
            json!({"value": 4}),
        )
        .await;

        Ok(())
    }

    // Test that we can launch a single task and listen for
    // the status changes when reading the input values from
    // a FileAssetStorage
    #[rstest]
    #[case("memory")]
    #[case("file")]
    #[tokio::test]
    async fn execute_inmemory_with_file_inputs(
        #[case] default_storage_name: &str,
    ) -> miette::Result<()> {
        let (registry, input_sets, _dir) =
            test_storage_registry(vec![], vec![json!({"a": 1, "b": 3})]).await;

        let task_plans = vec![TaskPlan {
            worker_name: "builtin".to_string(),
            task_name: "iadd".to_string(),

            inputs: input_sets[0].clone(),

            ..Default::default()
        }];
        let executor = InMemoryExecutor::try_new(&registry, default_storage_name).await?;

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
            default_storage_name,
            &events[1].clone().outputs()[0],
            json!({"value": 4}),
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
    async fn execute_inmemory_two_tasks(#[case] default_storage_name: &str) -> miette::Result<()> {
        let (registry, input_sets, _dir) = test_storage_registry(
            vec![json!({"a": 1, "b": 3}), json!({"a": 4, "b": 8})],
            vec![],
        )
        .await;

        let loc1 = Location::from_usize_iter([0]);
        let loc2 = Location::from_usize_iter([1]);
        let task_plans = vec![
            TaskPlan {
                loc: loc1.clone(),
                worker_name: "builtin".to_string(),
                task_name: "iadd".to_string(),

                inputs: input_sets[0].clone(),

                ..Default::default()
            },
            TaskPlan {
                loc: loc2.clone(),
                worker_name: "builtin".to_string(),
                task_name: "iadd".to_string(),

                inputs: input_sets[1].clone(),

                ..Default::default()
            },
        ];
        let executor = InMemoryExecutor::try_new(&registry, default_storage_name).await?;

        let stream = executor.listen()?;
        executor.execute(task_plans).await?;

        let events = stream.take(4).collect::<Vec<_>>().await;
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
            default_storage_name,
            &complete0.clone().outputs()[0],
            json!({"value": 4}),
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
            default_storage_name,
            &complete1.clone().outputs()[0],
            json!({"value": 12}),
        )
        .await;

        Ok(())
    }

    // Test that we can launch a task and listen for
    // the status changes even if we launch the task before
    // we start listening to the executor.
    #[tokio::test]
    async fn execute_inmemory_execute_before_listen() -> miette::Result<()> {
        let (registry, input_sets, _) =
            test_storage_registry(vec![json!({"a": 1, "b": 3})], vec![]).await;

        let task_plans = vec![TaskPlan {
            worker_name: "builtin".to_string(),
            task_name: "iadd".to_string(),

            inputs: input_sets[0].clone(),

            ..Default::default()
        }];
        let executor = InMemoryExecutor::try_new(&registry, "memory").await?;

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
            "memory",
            &events[1].clone().outputs()[0],
            json!({"value": 4}),
        )
        .await;

        Ok(())
    }

    // Test that we can launch a task and listen for
    // errors when they occur
    #[tokio::test]
    async fn execute_inmemory_error() -> miette::Result<()> {
        let (registry, _, _) = test_storage_registry(vec![], vec![]).await;
        let task_plans = vec![TaskPlan {
            worker_name: "builtin".to_string(),
            // builtin-worker has no task called "backflip"
            task_name: "backflip".to_string(),

            inputs: HashMap::new(),

            ..Default::default()
        }];
        let executor = InMemoryExecutor::try_new(&registry, "memory").await?;

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
                    status: NodeStatus::Error { ref error, .. },
                    ..
                }),
                ..
            } if error == "Unknown task: `backflip`"
        ));

        Ok(())
    }

    // Test that we can launch a task and then cancel it
    // before it completes.
    #[tokio::test]
    async fn execute_inmemory_cancel() -> miette::Result<()> {
        let (registry, input_sets, _) =
            test_storage_registry(vec![json!({"delay_seconds": 1})], vec![]).await;

        let loc = Location::from_usize_iter([0]);
        let task_plans = vec![TaskPlan {
            loc: loc.clone(),
            worker_name: "builtin".to_string(),
            task_name: "sleep".to_string(),

            inputs: input_sets[0].clone(),

            ..Default::default()
        }];
        let executor = InMemoryExecutor::try_new(&registry, "memory").await?;

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
    async fn execute_inmemory_cancel_non_existent() -> miette::Result<()> {
        let (registry, _, _) = test_storage_registry(vec![], vec![]).await;
        let executor = InMemoryExecutor::try_new(&registry, "memory").await?;

        let loc = Location::from_usize_iter([0]);
        executor.cancel(Uuid::nil(), 0, vec![loc]).await?;

        Ok(())
    }

    // Test that we can pass a completed ID to cancel and
    // it will not error.
    #[tokio::test]
    async fn execute_inmemory_cancel_completed() -> miette::Result<()> {
        let (registry, input_sets, _) =
            test_storage_registry(vec![json!({"a": 1, "b": 3})], vec![]).await;

        let loc = Location::from_usize_iter([0]);
        let task_plans = vec![TaskPlan {
            loc: loc.clone(),
            worker_name: "builtin".to_string(),
            task_name: "iadd".to_string(),

            inputs: input_sets[0].clone(),

            ..Default::default()
        }];
        let executor = InMemoryExecutor::try_new(&registry, "memory").await?;

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
            "memory",
            &events[1].clone().outputs()[0],
            json!({"value": 4}),
        )
        .await;

        executor.cancel(Uuid::nil(), 0, vec![loc]).await?;

        Ok(())
    }
}
