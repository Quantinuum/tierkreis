/*!
This module defines the [`InMemoryExecutor`] struct which implements [Executor]
by running small tasks from a small work queue.
*/
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use futures::{
    FutureExt, SinkExt, StreamExt,
    channel::mpsc,
    future::BoxFuture,
    stream::{BoxStream, FuturesUnordered},
};
use miette::{IntoDiagnostic, miette};
use tokio::task::{AbortHandle, JoinHandle};
use uuid::Uuid;

use crate::{
    asset_storage::{AssetStorageRegistry, load_assets, save_assets},
    builtins,
    event::{
        EventReceiver, EventSender, RuntimeEvent, send_cancelled, send_complete, send_error,
        send_running,
    },
    executor::{
        Executor,
        interface::{TaskPlan, WorkerSpec},
    },
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

type RunningFutures = FuturesUnordered<JoinHandle<BackgroundTask>>;
type AbortHandles = HashMap<(Uuid, u32, Location), AbortHandle>;

async fn process_cancelled_task(
    event_sender: &mut EventSender,
    abort_handles: &mut AbortHandles,
    workflow_run_id: Uuid,
    attempt: u32,
    loc: Location,
) -> miette::Result<()> {
    if let Some(handle) = abort_handles.remove(&(workflow_run_id, attempt, loc.clone())) {
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
) -> miette::Result<()> {
    let loc = background_task.loc;
    let task_outputs = background_task.task_output;
    let output_storage_name = background_task.output_storage_name;
    let workflow_run_id = background_task.workflow_run_id;
    let attempt = background_task.attempt;

    abort_handles.remove(&(workflow_run_id, attempt, loc.clone()));

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
            send_complete(event_sender, workflow_run_id, attempt, vec![(loc, outputs)]).await?;
        }
        Err(err) => send_error(event_sender, workflow_run_id, attempt, loc, &err).await?,
    }

    Ok(())
}

async fn start_task(
    event_sender: &mut EventSender,
    abort_handles: &mut AbortHandles,
    asset_storage_registry: &AssetStorageRegistry,
    running: &mut RunningFutures,
    internal_task: BackgroundTaskPlan,
) -> miette::Result<()> {
    // InMemory doesn't need to check if this is already running
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
                task_output: builtins::execute(&task_name, &inputs),
            })
        })
    });

    abort_handles.insert((workflow_run_id, attempt, loc.clone()), task.abort_handle());
    running.push(task);

    Ok(())
}

async fn process_tasks(
    mut task_receiver: TaskReceiver,
    mut cancel_receiver: CancelReceiver,
    mut event_sender: EventSender,
    asset_storage_registry: AssetStorageRegistry,
    mut abort_handles: AbortHandles,
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
                tracing::debug!(
                    workflow_run_id = %background_task.workflow_run_id,
                    attempt = %background_task.attempt,
                    loc = %background_task.loc,
                    "Task completed"
                );
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
                tracing::debug!(
                    workflow_run_id = %internal_task.task_plan.workflow_run_id,
                    attempt = %internal_task.task_plan.attempt,
                    loc = %internal_task.task_plan.loc,
                    "Received task {}",
                    internal_task.task_plan.task_name
                );
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
    background_abort_handle: AbortHandle,
    output_storage_name: String,
}

impl Drop for InMemoryExecutor {
    fn drop(&mut self) {
        self.background_abort_handle.abort();
    }
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
        if !asset_storage_registry.contains_key(output_storage_name) {
            return Err(miette!("output_storage_name not in registry"));
        }

        let asset_storage_registry = Arc::clone(asset_storage_registry);

        let (task_sender, task_receiver) = mpsc::channel(64);
        let (event_sender, event_receiver) = mpsc::channel(64);
        let (cancel_sender, cancel_receiver) = mpsc::channel(64);
        let abort_handles = HashMap::new();
        let background_task = tokio::spawn(Box::pin(process_tasks(
            task_receiver,
            cancel_receiver,
            event_sender,
            asset_storage_registry,
            abort_handles,
        )));

        Ok(Self {
            task_sender,
            cancel_sender,
            event_receiver: Mutex::new(Some(event_receiver)),
            background_abort_handle: background_task.abort_handle(),
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

            for mut task_plan in task_plans {
                let output_storage_name = task_plan
                    .output_storage_name
                    .take()
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
}

#[cfg(test)]
mod tests {
    use std::assert_matches;

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
            test_storage_registry(vec![json!({"a": 1, "b": 3})], vec![]).await;

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
        assert_matches!(
            events[0],
            RuntimeEvent::WorkflowRun {
                event: WorkflowRunEvent::NodeEvents(ref node_events),
                ..
            } if matches!(node_events[0].status, NodeStatus::Running { .. })
        );
        assert_matches!(
            events[1],
            RuntimeEvent::WorkflowRun {
                event: WorkflowRunEvent::NodeEvents(ref node_events),
                ..
            } if matches!(node_events[0].status, NodeStatus::Complete { .. })
        );
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
        let executor = InMemoryExecutor::try_new(&registry, default_storage_name)?;

        let stream = executor.listen()?;
        executor.execute(task_plans).await?;

        let events = stream.take(2).collect::<Vec<_>>().await;
        assert_eq!(events.len(), 2);
        assert_matches!(
            events[0],
            RuntimeEvent::WorkflowRun {
                event: WorkflowRunEvent::NodeEvents(ref node_events),
                ..
            } if matches!(node_events[0].status, NodeStatus::Running { .. })
        );
        assert_matches!(
            events[1],
            RuntimeEvent::WorkflowRun {
                event: WorkflowRunEvent::NodeEvents(ref node_events),
                ..
            } if matches!(node_events[0].status, NodeStatus::Complete { .. })
        );
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
        let executor = InMemoryExecutor::try_new(&registry, default_storage_name)?;

        let stream = executor.listen()?;
        executor.execute(task_plans).await?;
        let events = stream.take(4).collect::<Vec<_>>().await;
        assert_eq!(events.len(), 4);
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
                        event: WorkflowRunEvent::NodeEvents(node_events),
                        ..
                    } if node_events[0].loc == loc2
                    && matches!(node_events[0].status, NodeStatus::Complete { .. })
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
        let executor = InMemoryExecutor::try_new(&registry, "memory")?;

        executor.execute(task_plans).await?;
        let stream = executor.listen()?;

        let events = stream.take(2).collect::<Vec<_>>().await;
        assert_eq!(events.len(), 2);
        assert_matches!(
            events[0],
            RuntimeEvent::WorkflowRun {
                event: WorkflowRunEvent::NodeEvents(ref node_events),
                ..
            } if matches!(node_events[0].status, NodeStatus::Running { .. })
        );
        assert_matches!(
            events[1],
            RuntimeEvent::WorkflowRun {
                event: WorkflowRunEvent::NodeEvents(ref node_events),
                ..
            } if matches!(node_events[0].status, NodeStatus::Complete { .. })
        );
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
        let executor = InMemoryExecutor::try_new(&registry, "memory")?;

        let stream = executor.listen()?;
        executor.execute(task_plans).await?;

        let events = stream.take(2).collect::<Vec<_>>().await;
        assert_eq!(events.len(), 2);
        assert_matches!(
            events[0],
            RuntimeEvent::WorkflowRun {
                event: WorkflowRunEvent::NodeEvents(ref node_events),
                ..
            } if matches!(node_events[0].status, NodeStatus::Running { .. })
        );
        assert_matches!(
            events[1],
            RuntimeEvent::WorkflowRun {
                event: WorkflowRunEvent::NodeEvents(ref node_events),
                ..
            } if matches!(node_events[0].status, NodeStatus::Error { ref error, .. }
            if error == "Unknown task: `backflip`")
        );

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
        let executor = InMemoryExecutor::try_new(&registry, "memory")?;

        let mut stream = executor.listen()?;
        executor.execute(task_plans).await?;

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
    async fn execute_inmemory_cancel_non_existent() -> miette::Result<()> {
        let (registry, _, _) = test_storage_registry(vec![], vec![]).await;
        let executor = InMemoryExecutor::try_new(&registry, "memory")?;

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
        let executor = InMemoryExecutor::try_new(&registry, "memory")?;

        let stream = executor.listen()?;
        executor.execute(task_plans).await?;

        let events = stream.take(2).collect::<Vec<_>>().await;
        assert_eq!(events.len(), 2);
        assert_matches!(
            events[0],
            RuntimeEvent::WorkflowRun {
                event: WorkflowRunEvent::NodeEvents(ref node_events),
                ..
            } if matches!(node_events[0].status, NodeStatus::Running { .. })
        );
        assert_matches!(
            events[1],
            RuntimeEvent::WorkflowRun {
                event: WorkflowRunEvent::NodeEvents(ref node_events),
                ..
            } if matches!(node_events[0].status, NodeStatus::Complete { .. })
        );
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
