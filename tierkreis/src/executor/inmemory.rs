use std::{
    collections::{BTreeMap, HashMap, VecDeque},
    sync::{
        Arc, Mutex,
        atomic::{AtomicU32, Ordering},
    },
};

use futures::{
    FutureExt, StreamExt,
    future::BoxFuture,
    stream::{AbortHandle, Abortable, BoxStream},
};
use miette::miette;
use serde_json::Value;

use crate::{
    asset_storage::{AssetStorageRegistry, load_inputs, save_outputs},
    executor::interface::{Event, Executor, Status, TaskPlan, WorkerSpec},
};

#[derive(Clone, Debug, PartialEq, Default)]
pub struct InMemoryResourceSpec {}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct InMemoryEnvironmentSpec {}

#[derive(Clone, Debug, PartialEq, Default)]
struct TaskInfo {
    pub id: u32,
    pub output_storage_name: String,
}

type TaskFuture<'a> = BoxFuture<'a, miette::Result<HashMap<String, Value>>>;
type AbortHandles = Arc<Mutex<BTreeMap<u32, AbortHandle>>>;
type WorkQueue<'a> = Arc<Mutex<VecDeque<(TaskInfo, Abortable<TaskFuture<'a>>)>>>;

pub struct InMemoryExecutor<'a> {
    id_source: AtomicU32,
    work_queue: WorkQueue<'a>,
    abort_handles: AbortHandles,

    output_storage_name: String,
    asset_storage_registry: AssetStorageRegistry,
}

impl<'a> InMemoryExecutor<'a> {
    pub fn try_new(
        asset_storage_registry: &AssetStorageRegistry,
        output_storage_name: &str,
    ) -> miette::Result<Self> {
        let asset_storage_registry_lock = asset_storage_registry.read().unwrap();
        if !asset_storage_registry_lock.contains_key(output_storage_name) {
            return Err(miette!("output_storage_name not in registry"));
        }

        let work_queue = Arc::new(Mutex::new(VecDeque::new()));
        let abort_handles = Arc::new(Mutex::new(BTreeMap::new()));
        let asset_storage_registry = Arc::clone(asset_storage_registry);

        Ok(Self {
            id_source: AtomicU32::new(0),
            work_queue,
            abort_handles,
            output_storage_name: output_storage_name.to_string(),
            asset_storage_registry,
        })
    }

    async fn run_builtin(
        task_name: String,
        inputs: HashMap<String, Value>,
    ) -> miette::Result<HashMap<String, Value>> {
        let mut out = HashMap::new();
        match &*task_name {
            "iadd" => {
                let a = inputs.get("a").ok_or(miette!("Missing input: a"))?;
                let b = inputs.get("b").ok_or(miette!("Missing input: b"))?;

                let a = if let Value::Number(a) = a {
                    if let Some(a) = a.as_i64() {
                        Ok(a)
                    } else {
                        Err(miette!("Range error: {} is not representable as i64", a))
                    }
                } else {
                    Err(miette!("Type error: {} is not a number", a))
                }?;

                let b = if let Value::Number(b) = b {
                    if let Some(b) = b.as_i64() {
                        Ok(b)
                    } else {
                        Err(miette!("Range error: {} is not representable as i64", b))
                    }
                } else {
                    Err(miette!("Type error: {} is not a number", b))
                }?;

                out.insert("value".to_string(), Value::Number((a + b).into()));

                Ok(out)
            }
            _ => Err(miette!("Unknown task")),
        }
    }

    // Internal implementation for the Executor trait so that we can use `async fn` syntax
    // before we need to Box the result for the trait.
    async fn execute(&self, task_plans: Vec<TaskPlan>) -> miette::Result<Vec<u32>> {
        let mut ids = Vec::new();

        let mut work_queue = self
            .work_queue
            .lock()
            .map_err(|err| miette!("Failed to lock work queue: {}", err))?;
        let mut abort_handles = self
            .abort_handles
            .lock()
            .map_err(|err| miette!("Failed to lock abort handles: {}", err))?;

        for task_plan in task_plans.into_iter() {
            let id = self.id_source.fetch_add(1, Ordering::Relaxed);

            let inputs = load_inputs(&self.asset_storage_registry, task_plan.inputs.clone())?;

            let fut = match &*task_plan.worker_name {
                "builtin" => Ok(Self::run_builtin(task_plan.task_name.clone(), inputs)),
                _ => Err(miette!("Unknown worker name: {}", task_plan.worker_name)),
            }?;

            let (abort_handle, abort_registration) = AbortHandle::new_pair();
            ids.push(id);
            work_queue.push_back((
                TaskInfo {
                    id,
                    output_storage_name: task_plan
                        .output_storage_name
                        .unwrap_or_else(|| self.output_storage_name.clone()),
                },
                Abortable::new(fut.boxed(), abort_registration),
            ));
            abort_handles.insert(id, abort_handle);
        }

        Ok(ids)
    }
}

impl<'a> Executor for InMemoryExecutor<'a> {
    fn workers(&self) -> BoxFuture<'_, miette::Result<Vec<WorkerSpec>>> {
        futures::future::ok(vec![WorkerSpec {
            worker_name: "builtin".to_string(),
        }])
        .boxed()
    }

    fn execute(&self, task_plans: Vec<TaskPlan>) -> BoxFuture<'_, miette::Result<Vec<u32>>> {
        self.execute(task_plans).boxed()
    }

    fn listen(&self) -> miette::Result<BoxStream<'_, Event<u32>>> {
        Ok(InMemoryEventStream::new(&self.work_queue, &self.asset_storage_registry).boxed())
    }

    fn cancel(&self, task_ids: Vec<u32>) -> miette::Result<()> {
        let mut abort_handles = self
            .abort_handles
            .lock()
            .map_err(|err| miette!("Failed to lock abort handles: {}", err))?;
        for task_id in task_ids {
            if let Some(handle) = abort_handles.remove(&task_id) {
                handle.abort();
            }
        }
        Ok(())
    }
}

pub struct InMemoryEventStream<'a> {
    work_queue: WorkQueue<'a>,
    asset_storage_registry: AssetStorageRegistry,
    running: bool,
}

impl<'a> InMemoryEventStream<'a> {
    fn new(work_queue: &WorkQueue<'a>, asset_storage_registry: &AssetStorageRegistry) -> Self {
        let work_queue = Arc::clone(work_queue);
        let asset_storage_registry = Arc::clone(asset_storage_registry);
        Self {
            work_queue,
            asset_storage_registry,
            running: false,
        }
    }
}

impl<'a> futures::Stream for InMemoryEventStream<'a> {
    type Item = super::interface::Event<u32>;

    fn poll_next(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        if !self.running {
            let task_info = {
                let work_queue_locked = self.work_queue.lock().unwrap();
                work_queue_locked
                    .front()
                    .map(|(task_info, _)| task_info.clone())
            };
            if let Some(TaskInfo { id, .. }) = task_info {
                self.running = true;
                return std::task::Poll::Ready(Some(Event {
                    id,
                    status: Status::Running,
                    detail: None,
                }));
            } else {
                cx.waker().wake_by_ref();
                return std::task::Poll::Pending;
            }
        }

        let first = {
            let mut work_queue_locked = self.work_queue.lock().unwrap();
            work_queue_locked.pop_front()
        };
        if let Some((
            TaskInfo {
                id,
                output_storage_name,
            },
            mut fut,
        )) = first
        {
            let res = fut.poll_unpin(cx);
            res.map(|res| {
                self.running = false;
                match res {
                    Ok(Ok(outputs)) => {
                        let outputs = save_outputs(
                            &self.asset_storage_registry,
                            &output_storage_name,
                            outputs,
                        );
                        match outputs {
                            Ok(outputs) => Some(Event {
                                id,
                                status: Status::Complete { outputs },
                                detail: None,
                            }),
                            Err(err) => Some(Event {
                                id,
                                status: Status::Error {
                                    error: err.to_string(),
                                },
                                detail: None,
                            }),
                        }
                    }
                    Ok(Err(err)) => Some(Event {
                        id,
                        status: Status::Error {
                            error: err.to_string(),
                        },
                        detail: None,
                    }),
                    Err(_aborted) => Some(Event {
                        id,
                        status: Status::Cancelled,
                        detail: None,
                    }),
                }
            })
        } else {
            cx.waker().wake_by_ref();
            std::task::Poll::Pending
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let work_queue = self.work_queue.lock().unwrap();
        (work_queue.len(), None)
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
        assert!(events[1].is_complete());
        assert_registry_contains_values(
            &registry,
            default_storage_name,
            &events[1].clone().outputs().unwrap(),
            json!({"value": 4}),
        );

        assert_eq!(events[2].status, Status::Running);
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
                error: "Unknown task".to_string()
            }
        );

        Ok(())
    }

    // Test that we can launch a task and then cancel it
    // before it completes.
    #[tokio::test]
    async fn execute_inmemory_cancel() -> miette::Result<()> {
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
        executor.cancel(task_ids)?;

        let events = stream.take(2).collect::<Vec<_>>().await;
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].status, Status::Running);
        assert_eq!(events[1].status, Status::Cancelled);

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
