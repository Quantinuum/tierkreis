/*!
This module defines the [`InMemoryExecutor`] struct which implements [Executor]
by running small tasks from a small work queue.
*/
use std::{
    collections::HashMap,
    sync::{
        Arc, Mutex,
        atomic::{AtomicU32, Ordering},
    },
    thread::sleep,
    time::Duration,
};

use futures::{
    FutureExt, StreamExt,
    channel::mpsc,
    future::{self, BoxFuture},
    stream::{BoxStream, FuturesUnordered},
};
use miette::{IntoDiagnostic, miette};
use serde_json::Value;
use tokio::task::{AbortHandle, JoinHandle};

use crate::{
    asset_storage::{AssetStorageRegistry, load_inputs, save_outputs},
    event::{Event, Status},
    executor::interface::{Executor, TaskPlan, WorkerSpec},
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

type TaskSender = mpsc::Sender<(u32, TaskPlan, String)>;
type TaskReceiver = mpsc::Receiver<(u32, TaskPlan, String)>;
type EventSender = mpsc::Sender<Event>;
type EventReceiver = mpsc::Receiver<Event>;
type CancelSender = mpsc::Sender<u32>;
type CancelReceiver = mpsc::Receiver<u32>;

fn extract_i64(inputs: &HashMap<String, Vec<u8>>, name: &str) -> miette::Result<i64> {
    let asset = inputs.get(name).ok_or(miette!("Missing input: {name}"))?;
    let val = serde_json::from_slice(asset).into_diagnostic()?;

    if let Value::Number(val) = val {
        if let Some(val) = val.as_i64() {
            Ok(val)
        } else {
            Err(miette!("Range error: {val} is not representable as i64"))
        }
    } else {
        Err(miette!("Type error: {val} is not a Number"))
    }
}

fn extract_f64(inputs: &HashMap<String, Vec<u8>>, name: &str) -> miette::Result<f64> {
    let asset = inputs.get(name).ok_or(miette!("Missing input: {name}"))?;
    let val = serde_json::from_slice(asset).into_diagnostic()?;

    if let Value::Number(val) = val {
        if let Some(val) = val.as_f64() {
            Ok(val)
        } else {
            Err(miette!("Range error: {val} is not representable as f64"))
        }
    } else {
        Err(miette!("Type error: {val} is not a Number"))
    }
}

fn output_value(
    outputs: &mut HashMap<String, Vec<u8>>,
    value: impl Into<Value>,
) -> miette::Result<()> {
    let value: Value = value.into();
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
            let a = extract_i64(inputs, "a")?;
            let b = extract_i64(inputs, "b")?;

            output_value(&mut outputs, a + b)?;
        }
        "isub" => {
            let a = extract_i64(inputs, "a")?;
            let b = extract_i64(inputs, "b")?;

            output_value(&mut outputs, a - b)?;
        }
        "sleep" => {
            let delay_seconds = extract_f64(inputs, "delay_seconds")?;
            sleep(Duration::from_secs_f64(delay_seconds));

            output_value(&mut outputs, true)?;
        }
        _ => return Err(miette!("Unknown task")),
    }
    Ok(outputs)
}

fn send_error(event_sender: &mut EventSender, id: u32, err: miette::Error) {
    event_sender
        .try_send(Event {
            id,
            status: Status::Error {
                error: err.to_string(),
                detail: None,
            },
        })
        .expect("Failed to send update");
}

type RunningFutures =
    FuturesUnordered<JoinHandle<(u32, String, Result<HashMap<String, Vec<u8>>, miette::Error>)>>;

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
            Some(id) = cancel_receiver.next() => {
                let handle = abort_handles.remove(&id);
                if let Some(handle) = handle {
                    handle.abort();
                    event_sender
                        .try_send(Event {
                            id,
                            status: Status::Cancelled {},
                        })
                        .expect("Failed to send update");
                }
            }
            Some(res) = running.next() => {
                let (id, output_storage_name, outputs) = match res {
                    Ok(ok) => ok,
                    Err(err) => panic!("Failed to join to future: {err}"),
                };

                abort_handles.remove(&id);

                let outputs = match outputs {
                    Ok(outputs) => outputs,
                    Err(err) => {
                        send_error(&mut event_sender, id, err);
                        continue;
                    }
                };
                let outputs = save_outputs(&asset_storage_registry, &output_storage_name, outputs);
                match outputs {
                    Ok(outputs) => event_sender
                        .try_send(Event {
                            id,
                            status: Status::Complete { outputs },
                        })
                        .expect("Failed to send update"),
                    Err(err) => send_error(&mut event_sender, id, err),
                }
            }
            Some((id, task_plan, output_storage_name)) = task_receiver.next() => {
                event_sender
                    .try_send(Event {
                        id,
                        status: Status::Running {},
                    })
                    .expect("Failed to send update");

                let res = load_inputs(&asset_storage_registry, task_plan.inputs);

                let inputs = match res {
                    Ok(inputs) => inputs,
                    Err(err) => {
                        send_error(&mut event_sender, id, err);
                        continue;
                    }
                };

                let task = tokio::task::spawn_blocking(move || {
                    let inputs = inputs;
                    let task_name = task_plan.task_name;
                    (id, output_storage_name, run_builtin(&task_name, &inputs))
                });

                abort_handles.insert(id, task.abort_handle());
                running.push(task);
            }
        }
    }
}

/// [`InMemoryExecutor`] defines an [Executor] that performs Task Nodes in the same runtime.
///
/// These Tasks should be short lived Tasks where we want to avoid the overhead of spinning
/// up an entirely new process.
pub struct InMemoryExecutor {
    id_source: AtomicU32,

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
        tokio::spawn(process_tasks(
            task_receiver,
            cancel_receiver,
            event_sender,
            asset_storage_registry,
        ));

        Ok(Self {
            id_source: AtomicU32::new(0),

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

    fn execute(&self, task_plans: Vec<TaskPlan>) -> BoxFuture<'_, miette::Result<Vec<u32>>> {
        let res = || {
            let mut ids = Vec::new();
            let mut task_sender = self.task_sender.clone();

            for task_plan in task_plans {
                let id = self.id_source.fetch_add(1, Ordering::Relaxed);
                let output_storage_name = task_plan
                    .output_storage_name
                    .clone()
                    .unwrap_or_else(|| self.output_storage_name.clone());

                task_sender
                    .try_send((id, task_plan, output_storage_name))
                    .into_diagnostic()?;

                ids.push(id);
            }
            Ok(ids)
        };

        future::ready(res()).boxed()
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
        let mut cancel_sender = self.cancel_sender.clone();
        for task_id in task_ids {
            cancel_sender.try_send(task_id).into_diagnostic()?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use futures::StreamExt;
    use rstest::rstest;
    use serde_json::json;

    use crate::{
        asset_storage::{assert_registry_contains_values, test_storage_registry},
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
        assert_eq!(events[0].status, Status::Running);
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
        assert_eq!(events[0].status, Status::Running);
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

        let task_plans = vec![
            TaskPlan {
                worker_name: "builtin".to_string(),
                task_name: "iadd".to_string(),

                inputs: input_sets[0].clone(),

                ..Default::default()
            },
            TaskPlan {
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
        assert_eq!(events[0].status, Status::Running);
        assert_eq!(events[1].status, Status::Running);

        assert!(events[2].is_complete());
        assert_registry_contains_values(
            &registry,
            default_storage_name,
            &events[2].clone().outputs().unwrap(),
            json!({"value": 4}),
        );

        assert!(events[3].is_complete());
        assert_registry_contains_values(
            &registry,
            default_storage_name,
            &events[3].clone().outputs().unwrap(),
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
        assert_eq!(events[0].status, Status::Running);
        assert!(events[1].is_complete());
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
        assert_eq!(events[0].status, Status::Running);
        assert_eq!(
            events[1].status,
            Status::Error {
                error: "Unknown task".to_string(),
                detail: None,
            }
        );

        Ok(())
    }

    // Test that we can launch a task and then cancel it
    // before it completes.
    #[tokio::test]
    async fn execute_inmemory_cancel() -> miette::Result<()> {
        let (registry, input_sets, _) =
            test_storage_registry(vec![json!({"delay_seconds": 1})], vec![]);

        let task_plans = vec![TaskPlan {
            worker_name: "builtin".to_string(),
            task_name: "sleep".to_string(),

            inputs: input_sets[0].clone(),

            ..Default::default()
        }];
        let executor = InMemoryExecutor::try_new(&registry, "memory")?;

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
    async fn execute_inmemory_cancel_non_existent() -> miette::Result<()> {
        let (registry, _, _) = test_storage_registry(vec![], vec![]);
        let executor = InMemoryExecutor::try_new(&registry, "memory")?;

        executor.cancel(vec![0])?;

        Ok(())
    }

    // Test that we can pass a completed ID to cancel and
    // it will not error.
    #[tokio::test]
    async fn execute_inmemory_cancel_completed() -> miette::Result<()> {
        let (registry, input_sets, _) =
            test_storage_registry(vec![json!({"a": 1, "b": 3})], vec![]);

        let task_plans = vec![TaskPlan {
            worker_name: "builtin".to_string(),
            task_name: "iadd".to_string(),

            inputs: input_sets[0].clone(),

            ..Default::default()
        }];
        let executor = InMemoryExecutor::try_new(&registry, "memory")?;

        let stream = executor.listen()?;
        let task_ids = executor.execute(task_plans).await?;

        let events = stream.take(2).collect::<Vec<_>>().await;
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].status, Status::Running);
        assert!(events[1].is_complete());
        assert_registry_contains_values(
            &registry,
            "memory",
            &events[1].clone().outputs().unwrap(),
            json!({"value": 4}),
        );

        executor.cancel(task_ids)?;

        Ok(())
    }
}
