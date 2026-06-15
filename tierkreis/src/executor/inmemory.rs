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

use crate::{
    asset_storage::{AssetStorageRegistry, load_assets, save_assets},
    event::{
        Event, EventReceiver, EventSender, send_cancelled, send_complete, send_error, send_running,
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
}

#[derive(Debug)]
struct BackgroundTask {
    loc: Location,
    output_storage_name: String,
    task_output: miette::Result<HashMap<String, Vec<u8>>>,
}

type TaskSender = mpsc::Sender<BackgroundTaskPlan>;
type TaskReceiver = mpsc::Receiver<BackgroundTaskPlan>;
type CancelSender = mpsc::Sender<Location>;
type CancelReceiver = mpsc::Receiver<Location>;

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
            let val: Option<T> = serde_json::from_slice(&asset).into_diagnostic()?;
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

fn run_builtin(
    task_name: &str,
    inputs: &HashMap<String, Vec<u8>>,
) -> miette::Result<HashMap<String, Vec<u8>>> {
    let mut outputs = HashMap::new();
    match task_name {
        "iadd" => {
            let a: i64 = extract_value(inputs, "a")?;
            let b: i64 = extract_value(inputs, "b")?;

            output_value(&mut outputs, a + b)?;
        }
        "isub" => {
            let a: i64 = extract_value(inputs, "a")?;
            let b: i64 = extract_value(inputs, "b")?;

            output_value(&mut outputs, a - b)?;
        }
        "itimes" => {
            let a: i64 = extract_value(inputs, "a")?;
            let b: i64 = extract_value(inputs, "b")?;

            output_value(&mut outputs, a * b)?;
        }
        "idivide" => {
            let a: i64 = extract_value(inputs, "a")?;
            let b: i64 = extract_value(inputs, "b")?;

            output_value(&mut outputs, a / b)?;
        }
        "igt" => {
            let a: i64 = extract_value(inputs, "a")?;
            let b: i64 = extract_value(inputs, "b")?;

            output_value(&mut outputs, a > b)?;
        }
        "mod" => {
            let a: i64 = extract_value(inputs, "a")?;
            let b: i64 = extract_value(inputs, "b")?;

            output_value(&mut outputs, a % b)?;
        }
        "eq" => {
            let a: Value = extract_value(inputs, "a")?;
            let b: Value = extract_value(inputs, "b")?;

            output_value(&mut outputs, a == b)?;
        }
        "neq" => {
            let a: Value = extract_value(inputs, "a")?;
            let b: Value = extract_value(inputs, "b")?;

            output_value(&mut outputs, a != b)?;
        }
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
            let step: Option<i64> = extract_optional_value(inputs, "step")?;

            let range = start..stop;

            let out = if let Some(step) = step {
                range.step_by(step as usize).collect::<Vec<_>>()
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

async fn process_cancelled_task(
    event_sender: &mut EventSender,
    abort_handles: &mut HashMap<Location, AbortHandle>,
    loc: Location,
) -> miette::Result<()> {
    let handle = abort_handles.remove(&loc);
    if let Some(handle) = handle {
        handle.abort();
        send_cancelled(event_sender, loc).await?;
    }
    Ok(())
}

async fn process_finished_task(
    event_sender: &mut EventSender,
    abort_handles: &mut HashMap<Location, AbortHandle>,
    asset_storage_registry: &AssetStorageRegistry,
    background_task: BackgroundTask,
) -> miette::Result<()> {
    let loc = background_task.loc;
    let task_outputs = background_task.task_output;
    let output_storage_name = background_task.output_storage_name;
    abort_handles.remove(&loc);

    let outputs = match task_outputs {
        Ok(outputs) => outputs,
        Err(err) => {
            send_error(event_sender, loc, &err).await?;
            return Ok(());
        }
    };

    let outputs = save_assets(asset_storage_registry, &output_storage_name, outputs);
    match outputs {
        Ok(outputs) => send_complete(event_sender, loc, outputs).await?,
        Err(err) => send_error(event_sender, loc, &err).await?,
    }

    Ok(())
}

async fn start_task(
    event_sender: &mut EventSender,
    abort_handles: &mut HashMap<Location, AbortHandle>,
    asset_storage_registry: &AssetStorageRegistry,
    running: &mut RunningFutures,
    internal_task: BackgroundTaskPlan,
) -> miette::Result<()> {
    let task_plan = internal_task.task_plan;
    let loc = task_plan.loc;
    let output_storage_name = internal_task.output_storage_name;
    send_running(event_sender, loc.clone()).await?;

    let res = load_assets(asset_storage_registry, &task_plan.inputs);

    let inputs = match res {
        Ok(inputs) => inputs,
        Err(err) => {
            send_error(event_sender, loc, &err).await?;
            return Ok(());
        }
    };

    let task_loc = loc.clone();
    let task = tokio::task::spawn_blocking(move || {
        let inputs = inputs;
        let task_name = task_plan.task_name;
        BackgroundTask {
            loc: task_loc,
            output_storage_name,
            task_output: run_builtin(&task_name, &inputs),
        }
    });

    abort_handles.insert(loc.clone(), task.abort_handle());
    running.push(task);

    Ok(())
}

async fn process_tasks(
    mut task_receiver: TaskReceiver,
    mut cancel_receiver: CancelReceiver,
    mut event_sender: EventSender,
    asset_storage_registry: AssetStorageRegistry,
) {
    let mut abort_handles: HashMap<Location, AbortHandle> = HashMap::new();
    let mut running: RunningFutures = FuturesUnordered::new();

    loop {
        tokio::select! {
            // A task has been cancelled
            Some(loc) = cancel_receiver.next() => {
                process_cancelled_task(&mut event_sender, &mut abort_handles, loc)
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
    pub fn try_new(
        asset_storage_registry: &AssetStorageRegistry,
        output_storage_name: &str,
    ) -> miette::Result<Self> {
        let asset_storage_registry_lock = asset_storage_registry
            .read()
            .map_err(|err| miette!("Failed to lock AssetStorageRegistry for reading: {err}"))?;
        if !asset_storage_registry_lock.contains_key(output_storage_name) {
            return Err(miette!("output_storage_name not in registry"));
        }

        let asset_storage_registry = Arc::clone(asset_storage_registry);

        let (task_sender, task_receiver) = mpsc::channel(64);
        let (event_sender, event_receiver) = mpsc::channel(64);
        let (cancel_sender, cancel_receiver) = mpsc::channel(64);
        tokio::spawn(Box::pin(process_tasks(
            task_receiver,
            cancel_receiver,
            event_sender,
            asset_storage_registry,
        )));

        Ok(Self {
            task_sender,
            cancel_sender,
            event_receiver: Mutex::new(Some(event_receiver)),
            output_storage_name: output_storage_name.to_string(),
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
                    })
                    .await
                    .into_diagnostic()?;
            }
            Ok(())
        };

        fut.boxed()
    }

    fn listen(&self) -> miette::Result<BoxStream<'static, Event>> {
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

    fn cancel(&self, task_locations: Vec<Location>) -> BoxFuture<'_, miette::Result<()>> {
        let mut cancel_sender = self.cancel_sender.clone();
        let fut = async move {
            for task_location in task_locations {
                cancel_sender.send(task_location).await.into_diagnostic()?;
            }
            Ok(())
        };
        fut.boxed()
    }
}

#[cfg(test)]
mod tests {
    use futures::StreamExt;
    use rstest::rstest;
    use serde_json::json;

    use crate::{
        asset_storage::{assert_registry_contains_values, test_storage_registry},
        event::{NodeEvent, NodeStatus},
        executor::interface::TaskPlan,
    };

    use super::*;

    #[tokio::test]
    async fn inmemory_workers() -> miette::Result<()> {
        let (registry, _, _) = test_storage_registry(vec![], vec![]);
        let executor = InMemoryExecutor::try_new(&registry, "memory")?;

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
            test_storage_registry(vec![json!({"a": 1, "b": 3})], vec![]);

        let task_plans = vec![TaskPlan {
            worker_name: "builtin".to_string(),
            task_name: "iadd".to_string(),

            inputs: input_sets[0].clone(),

            ..Default::default()
        }];
        let executor = InMemoryExecutor::try_new(&registry, default_storage_name)?;

        let stream = executor.listen()?;
        executor.execute(task_plans).await?;

        let events = stream.take(2).collect::<Vec<_>>().await;
        dbg!(&events);
        assert_eq!(events.len(), 2);
        assert!(matches!(
            events[0],
            Event::Node(NodeEvent {
                status: NodeStatus::Running { .. },
                ..
            })
        ));
        assert!(matches!(
            events[1],
            Event::Node(NodeEvent {
                status: NodeStatus::Complete { .. },
                ..
            })
        ));
        assert_registry_contains_values(
            &registry,
            default_storage_name,
            &events[1].clone().outputs().unwrap(),
            json!({"value": 4}),
        );

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
            test_storage_registry(vec![], vec![json!({"a": 1, "b": 3})]);

        let task_plans = vec![TaskPlan {
            worker_name: "builtin".to_string(),
            task_name: "iadd".to_string(),

            inputs: input_sets[0].clone(),

            ..Default::default()
        }];
        let executor = InMemoryExecutor::try_new(&registry, default_storage_name)?;

        let stream = executor.listen()?;
        executor.execute(task_plans).await?;

        let events = stream.take(2).collect::<Vec<_>>().await;
        assert_eq!(events.len(), 2);
        assert!(matches!(
            events[0],
            Event::Node(NodeEvent {
                status: NodeStatus::Running { .. },
                ..
            })
        ));
        assert!(matches!(
            events[1],
            Event::Node(NodeEvent {
                status: NodeStatus::Complete { .. },
                ..
            })
        ));
        assert_registry_contains_values(
            &registry,
            default_storage_name,
            &events[1].clone().outputs().unwrap(),
            json!({"value": 4}),
        );

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
        );

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
        let executor = InMemoryExecutor::try_new(&registry, default_storage_name)?;

        let stream = executor.listen()?;
        executor.execute(task_plans).await?;

        let events = stream.take(4).collect::<Vec<_>>().await;
        assert_eq!(events.len(), 4);
        assert!(events.contains(&Event::Node(NodeEvent {
            loc: loc1.clone(),
            status: NodeStatus::Running { state_update: None },
        })));
        assert!(events.contains(&Event::Node(NodeEvent {
            loc: loc2.clone(),
            status: NodeStatus::Running { state_update: None },
        })));

        // These may complete out of order, so find the correct events.
        let complete0 = events
            .iter()
            .find(|event| {
                matches!(
                    event,
                    Event::Node(NodeEvent {
                        loc,
                        status: NodeStatus::Complete { .. }
                    }) if loc == &loc1
                )
            })
            .unwrap();
        assert_registry_contains_values(
            &registry,
            default_storage_name,
            &complete0.clone().outputs().unwrap_or_default(),
            json!({"value": 4}),
        );
        let complete1 = events
            .iter()
            .find(|event| {
                matches!(
                    event,
                    Event::Node(NodeEvent {
                        loc,
                        status: NodeStatus::Complete { .. }
                    }) if loc == &loc2
                )
            })
            .unwrap();
        assert_registry_contains_values(
            &registry,
            default_storage_name,
            &complete1.clone().outputs().unwrap_or_default(),
            json!({"value": 12}),
        );

        Ok(())
    }

    // Test that we can launch a task and listen for
    // the status changes even if we launch the task before
    // we start listening to the executor.
    #[tokio::test]
    async fn execute_inmemory_execute_before_listen() -> miette::Result<()> {
        let (registry, input_sets, _) =
            test_storage_registry(vec![json!({"a": 1, "b": 3})], vec![]);

        let task_plans = vec![TaskPlan {
            worker_name: "builtin".to_string(),
            task_name: "iadd".to_string(),

            inputs: input_sets[0].clone(),

            ..Default::default()
        }];
        let executor = InMemoryExecutor::try_new(&registry, "memory")?;

        executor.execute(task_plans).await?;
        let stream = executor.listen()?;

        let events = stream.take(2).collect::<Vec<_>>().await;
        assert_eq!(events.len(), 2);
        assert!(matches!(
            events[0],
            Event::Node(NodeEvent {
                status: NodeStatus::Running { .. },
                ..
            })
        ));
        assert!(matches!(
            events[1],
            Event::Node(NodeEvent {
                status: NodeStatus::Complete { .. },
                ..
            })
        ));
        assert_registry_contains_values(
            &registry,
            "memory",
            &events[1].clone().outputs().unwrap(),
            json!({"value": 4}),
        );

        Ok(())
    }

    // Test that we can launch a task and listen for
    // errors when they occur
    #[tokio::test]
    async fn execute_inmemory_error() -> miette::Result<()> {
        let (registry, _, _) = test_storage_registry(vec![], vec![]);
        let task_plans = vec![TaskPlan {
            worker_name: "builtin".to_string(),
            // builtin-worker has no task called "backflip"
            task_name: "backflip".to_string(),

            inputs: HashMap::new(),

            ..Default::default()
        }];
        let executor = InMemoryExecutor::try_new(&registry, "memory")?;

        let stream = executor.listen()?;
        executor.execute(task_plans).await?;

        let events = stream.take(2).collect::<Vec<_>>().await;
        assert_eq!(events.len(), 2);
        assert!(matches!(
            events[0],
            Event::Node(NodeEvent {
                status: NodeStatus::Running { .. },
                ..
            })
        ));
        dbg!(&events[1]);
        assert!(matches!(
            &events[1],
            Event::Node(NodeEvent {
                status: NodeStatus::Error {
                    error,
                    ..
                },
                ..
            }) if error == "Unknown task: `backflip`"
        ));

        Ok(())
    }

    // Test that we can launch a task and then cancel it
    // before it completes.
    #[tokio::test]
    async fn execute_inmemory_cancel() -> miette::Result<()> {
        let (registry, input_sets, _) =
            test_storage_registry(vec![json!({"delay_seconds": 1})], vec![]);

        let loc = Location::from_usize_iter([0]);
        let task_plans = vec![TaskPlan {
            loc: loc.clone(),
            worker_name: "builtin".to_string(),
            task_name: "sleep".to_string(),

            inputs: input_sets[0].clone(),

            ..Default::default()
        }];
        let executor = InMemoryExecutor::try_new(&registry, "memory")?;

        let mut stream = executor.listen()?;
        executor.execute(task_plans).await?;

        let event = stream.next().await.unwrap();
        assert!(matches!(
            event,
            Event::Node(NodeEvent {
                status: NodeStatus::Running { .. },
                ..
            })
        ));

        executor.cancel(vec![loc]).await?;

        let event = stream.next().await.unwrap();
        assert!(matches!(
            event,
            Event::Node(NodeEvent {
                status: NodeStatus::Cancelled,
                ..
            })
        ));

        Ok(())
    }

    // Test that we can pass a non-existent ID to cancel and
    // it will not error.
    #[tokio::test]
    async fn execute_inmemory_cancel_non_existent() -> miette::Result<()> {
        let (registry, _, _) = test_storage_registry(vec![], vec![]);
        let executor = InMemoryExecutor::try_new(&registry, "memory")?;

        let loc = Location::from_usize_iter([0]);
        executor.cancel(vec![loc]).await?;

        Ok(())
    }

    // Test that we can pass a completed ID to cancel and
    // it will not error.
    #[tokio::test]
    async fn execute_inmemory_cancel_completed() -> miette::Result<()> {
        let (registry, input_sets, _) =
            test_storage_registry(vec![json!({"a": 1, "b": 3})], vec![]);

        let loc = Location::from_usize_iter([0]);
        let task_plans = vec![TaskPlan {
            loc: loc.clone(),
            worker_name: "builtin".to_string(),
            task_name: "iadd".to_string(),

            inputs: input_sets[0].clone(),

            ..Default::default()
        }];
        let executor = InMemoryExecutor::try_new(&registry, "memory")?;

        let stream = executor.listen()?;
        executor.execute(task_plans).await?;

        let events = stream.take(2).collect::<Vec<_>>().await;
        assert_eq!(events.len(), 2);
        assert!(matches!(
            events[0],
            Event::Node(NodeEvent {
                status: NodeStatus::Running { .. },
                ..
            })
        ));
        assert!(matches!(
            events[1],
            Event::Node(NodeEvent {
                status: NodeStatus::Complete { .. },
                ..
            })
        ));
        assert_registry_contains_values(
            &registry,
            "memory",
            &events[1].clone().outputs().unwrap(),
            json!({"value": 4}),
        );

        executor.cancel(vec![loc]).await?;

        Ok(())
    }
}
