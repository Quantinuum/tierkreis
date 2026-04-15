use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use futures::{
    StreamExt,
    channel::mpsc,
    stream::{BoxStream, select_all},
};
use miette::{IntoDiagnostic, miette};
use serde_json::Value;

use crate::{
    asset_storage::{
        AssetStorageRegistry,
        interface::{AssetKey, AssetSpec},
        transfer_assets,
    },
    executor::{
        ExecutorRegistry,
        interface::{Event, Status, TaskPlan},
    },
};

pub enum Node {
    Task {
        worker_name: String,
        task_name: String,
    },
    Input {
        name: String,
    },
    Output {},
    Const {
        value: Value,
    },
    Eval {},
}

pub struct Orchestrator {
    event_sender: mpsc::Sender<Event<u32>>,
    event_receiver: Mutex<Option<mpsc::Receiver<Event<u32>>>>,

    default_executor_name: String,
    executor_registry: ExecutorRegistry,

    default_storage_name: String,
    asset_storage_registry: AssetStorageRegistry,
}

impl Orchestrator {
    pub fn try_new(
        asset_storage_registry: &AssetStorageRegistry,
        executor_registry: &ExecutorRegistry,
        default_storage_name: &str,
        default_executor_name: &str,
    ) -> miette::Result<Self> {
        let (sender, receiver) = mpsc::channel(128);

        let asset_storage_registry_lock = asset_storage_registry.read().unwrap();
        if !asset_storage_registry_lock.contains_key(default_storage_name) {
            return Err(miette!("default_storage_name not in registry"));
        }

        if !executor_registry.contains_key(default_executor_name) {
            return Err(miette!("default_executor_name not in registry"));
        }

        Ok(Self {
            event_sender: sender,
            event_receiver: Mutex::new(Some(receiver)),

            default_executor_name: default_executor_name.to_string(),
            executor_registry: Arc::clone(executor_registry),

            default_storage_name: default_storage_name.to_string(),
            asset_storage_registry: Arc::clone(asset_storage_registry),
        })
    }

    pub async fn start_node(
        &self,
        node: &Node,
        inputs: HashMap<String, AssetSpec>,
    ) -> miette::Result<()> {
        match node {
            Node::Task {
                worker_name,
                task_name,
            } => {
                let task_plans = vec![TaskPlan {
                    worker_name: worker_name.clone(),
                    task_name: task_name.clone(),

                    inputs,

                    output_storage_name: Some(self.default_storage_name.clone()),
                    ..Default::default()
                }];
                let executor = self
                    .executor_registry
                    .get(&self.default_executor_name)
                    .unwrap();
                executor.execute(task_plans).await?;
            }
            Node::Input { name } => {
                // Assume the inputs are the inputs to the graph.
                //
                // Just pull out the named input and assign it to the "value" output.
                let value_spec = inputs.get(name).ok_or(miette!("Missing input: {name}"))?;
                let mut outputs = HashMap::new();
                outputs.insert("value".to_string(), value_spec.clone());

                // Move the value into storage if needed.
                if value_spec.storage_name != self.default_storage_name {
                    outputs = transfer_assets(
                        &self.asset_storage_registry,
                        &self.default_storage_name,
                        outputs,
                    )?;
                }

                let mut event_sender = self.event_sender.clone();
                event_sender
                    .try_send(Event {
                        id: 0,
                        status: Status::Complete { outputs },
                    })
                    .into_diagnostic()?;
            }
            Node::Output {} => {
                // Notify that the outputs are ready
                let mut event_sender = self.event_sender.clone();

                let outputs = transfer_assets(
                    &self.asset_storage_registry,
                    &self.default_storage_name,
                    inputs,
                )?;
                event_sender
                    .try_send(Event {
                        id: 0,
                        status: Status::Complete { outputs },
                    })
                    .into_diagnostic()?;
            }
            Node::Const { value } => {
                // Load the constant value into the correct storage.
                let asset_storage_registry = self.asset_storage_registry.read().unwrap();
                let asset_storage = asset_storage_registry
                    .get(&self.default_storage_name)
                    .unwrap();
                let asset_key = AssetKey::new();
                asset_storage.save(&asset_key, value.clone())?;

                let mut outputs = HashMap::new();
                outputs.insert(
                    "value".to_string(),
                    AssetSpec {
                        kind: asset_storage.kind(),
                        storage_name: self.default_storage_name.clone(),
                        asset_key,
                    },
                );

                let mut event_sender = self.event_sender.clone();
                event_sender
                    .try_send(Event {
                        id: 0,
                        status: Status::Complete { outputs },
                    })
                    .into_diagnostic()?;
            }
            Node::Eval {} => {
                // spawn a sub-orchestrator?
            }
        }

        Ok(())
    }

    pub fn listen(&self) -> miette::Result<BoxStream<'_, Event<u32>>> {
        let orchestrator_events = {
            let mut receiver = self
                .event_receiver
                .try_lock()
                .map_err(|err| miette!("Failed to listen: {}", err))?;

            receiver.take().ok_or(miette!(
                "Failed to listen: Orchestrator is already being listened to."
            ))?
        };

        let mut streams = self
            .executor_registry
            .values()
            .map(|executor| {
                let stream = executor.listen()?;
                Ok(stream)
            })
            .collect::<miette::Result<Vec<BoxStream<Event<u32>>>>>()?;

        streams.push(orchestrator_events.boxed());
        Ok(select_all(streams).boxed())
    }
}

#[cfg(test)]
mod tests {
    use futures::StreamExt;
    use rstest::rstest;
    use serde_json::json;

    use crate::{
        asset_storage::{assert_registry_contains_values, test_storage_registry},
        executor::{
            inmemory::InMemoryExecutor, interface::Executor, subprocess::SubprocessExecutor,
        },
    };

    use super::*;

    fn test_executor_registry(asset_storage_registry: &AssetStorageRegistry) -> ExecutorRegistry {
        let mut executor_registry: HashMap<String, Box<dyn Executor>> = HashMap::new();

        executor_registry.insert(
            "memory".to_string(),
            Box::new(InMemoryExecutor::try_new(asset_storage_registry, "memory").unwrap()),
        );
        executor_registry.insert(
            "subprocess".to_string(),
            Box::new(SubprocessExecutor::try_new(asset_storage_registry, "file", "file").unwrap()),
        );

        Arc::new(executor_registry)
    }

    // Test that we can launch a single task and listen for
    // the status changes.
    #[rstest]
    #[case("memory")]
    #[case("file")]
    #[tokio::test]
    async fn start_task(#[case] default_storage_name: &str) -> miette::Result<()> {
        let (registry, input_sets, _dir) =
            test_storage_registry(vec![json!({"a": 1, "b": 3})], vec![]);
        let executor_registry = test_executor_registry(&registry);
        let orchestrator = Orchestrator::try_new(
            &registry,
            &executor_registry,
            default_storage_name,
            "memory",
        )?;
        let stream = orchestrator.listen()?;

        let node = &Node::Task {
            worker_name: "builtin".to_string(),
            task_name: "iadd".to_string(),
        };
        let inputs = input_sets[0].clone();
        orchestrator.start_node(node, inputs).await?;

        let events = stream.take(2).collect::<Vec<_>>().await;
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].status, Status::Running);
        assert_registry_contains_values(
            &registry,
            &orchestrator.default_storage_name,
            &events[1].clone().outputs().unwrap(),
            json!({"value": 4}),
        );

        Ok(())
    }

    // Test that we can launch a single task and listen for
    // the status changes.
    #[rstest]
    #[case("memory")]
    #[case("file")]
    #[tokio::test]
    async fn start_input(#[case] default_storage_name: &str) -> miette::Result<()> {
        let (registry, input_sets, _dir) =
            test_storage_registry(vec![json!({"a": 1, "b": 3})], vec![]);
        let executor_registry = test_executor_registry(&registry);
        let orchestrator = Orchestrator::try_new(
            &registry,
            &executor_registry,
            default_storage_name,
            "memory",
        )?;
        let stream = orchestrator.listen()?;

        let node = &Node::Input {
            name: "a".to_string(),
        };
        let inputs = input_sets[0].clone();
        orchestrator.start_node(node, inputs).await?;

        let node = &Node::Input {
            name: "b".to_string(),
        };
        let inputs = input_sets[0].clone();
        orchestrator.start_node(node, inputs).await?;

        let events = stream.take(2).collect::<Vec<_>>().await;
        assert_eq!(events.len(), 2);
        assert_registry_contains_values(
            &registry,
            &orchestrator.default_storage_name,
            &events[0].clone().outputs().unwrap(),
            json!({"value": 1}),
        );
        assert_registry_contains_values(
            &registry,
            &orchestrator.default_storage_name,
            &events[1].clone().outputs().unwrap(),
            json!({"value": 3}),
        );

        Ok(())
    }

    // Test that we can launch a single task and listen for
    // the status changes.
    #[rstest]
    #[case("memory")]
    #[case("file")]
    #[tokio::test]
    async fn start_output(#[case] default_storage_name: &str) -> miette::Result<()> {
        let (registry, input_sets, _dir) =
            test_storage_registry(vec![json!({"a": 1, "b": 3})], vec![]);
        let executor_registry = test_executor_registry(&registry);
        let orchestrator = Orchestrator::try_new(
            &registry,
            &executor_registry,
            default_storage_name,
            "memory",
        )?;
        let stream = orchestrator.listen()?;

        let node = &Node::Output {};
        let inputs = input_sets[0].clone();
        orchestrator.start_node(node, inputs).await?;

        let events = stream.take(1).collect::<Vec<_>>().await;
        assert_eq!(events.len(), 1);
        assert_registry_contains_values(
            &registry,
            &orchestrator.default_storage_name,
            &events[0].clone().outputs().unwrap(),
            json!({"a": 1, "b": 3}),
        );

        Ok(())
    }

    // Test that we can launch a single task and listen for
    // the status changes.
    #[rstest]
    #[case("memory")]
    #[case("file")]
    #[tokio::test]
    async fn start_const(#[case] default_storage_name: &str) -> miette::Result<()> {
        let (registry, _input_sets, _dir) = test_storage_registry(vec![], vec![]);
        let executor_registry = test_executor_registry(&registry);
        let orchestrator = Orchestrator::try_new(
            &registry,
            &executor_registry,
            default_storage_name,
            "memory",
        )?;
        let stream = orchestrator.listen()?;

        let node = &Node::Const {
            value: "hello there".into(),
        };
        let inputs = HashMap::new();
        orchestrator.start_node(node, inputs).await?;

        let events = stream.take(1).collect::<Vec<_>>().await;
        assert_eq!(events.len(), 1);
        assert_registry_contains_values(
            &registry,
            &orchestrator.default_storage_name,
            &events[0].clone().outputs().unwrap(),
            json!({"value": "hello there"}),
        );

        Ok(())
    }

    // Test that we can launch a single task and listen for
    // the status changes.
    #[rstest]
    #[case("memory")]
    #[case("file")]
    #[tokio::test]
    async fn start_simple_workflow(#[case] default_storage_name: &str) -> miette::Result<()> {
        let (registry, input_sets, _dir) = test_storage_registry(vec![json!({"a": 1})], vec![]);
        let executor_registry = test_executor_registry(&registry);
        let orchestrator = Orchestrator::try_new(
            &registry,
            &executor_registry,
            default_storage_name,
            "memory",
        )?;
        let mut stream = orchestrator.listen()?;

        let node = &Node::Input {
            name: "a".to_string(),
        };
        let inputs = input_sets[0].clone();
        orchestrator.start_node(node, inputs).await?;

        let input_complete_event = stream.next().await.unwrap();
        dbg!(&input_complete_event);
        let mut input_complete_outputs = input_complete_event.outputs().unwrap();
        assert_registry_contains_values(
            &registry,
            &orchestrator.default_storage_name,
            &input_complete_outputs,
            json!({"value": 1}),
        );

        let node = &Node::Const { value: 3.into() };
        let inputs = HashMap::new();
        orchestrator.start_node(node, inputs).await?;

        let const_complete_event = stream.next().await.unwrap();
        dbg!(&const_complete_event);
        let mut const_complete_outputs = const_complete_event.outputs().unwrap();
        assert_registry_contains_values(
            &registry,
            &orchestrator.default_storage_name,
            &const_complete_outputs,
            json!({"value": 3}),
        );

        let node = &Node::Task {
            worker_name: "builtin".to_string(),
            task_name: "iadd".to_string(),
        };
        let mut inputs = HashMap::new();
        inputs.insert(
            "a".to_string(),
            input_complete_outputs.remove("value").unwrap(),
        );
        inputs.insert(
            "b".to_string(),
            const_complete_outputs.remove("value").unwrap(),
        );
        orchestrator.start_node(node, inputs).await?;

        let task_running_event = stream.next().await.unwrap();
        dbg!(&task_running_event);
        assert_eq!(task_running_event.status, Status::Running);
        let task_complete_event = stream.next().await.unwrap();
        dbg!(&task_complete_event);
        let mut task_complete_outputs = task_complete_event.outputs().unwrap();
        assert_registry_contains_values(
            &registry,
            &orchestrator.default_storage_name,
            &task_complete_outputs,
            json!({"value": 4}),
        );

        let node = &Node::Output {};
        let mut inputs = HashMap::new();
        inputs.insert(
            "value".to_string(),
            task_complete_outputs.remove("value").unwrap(),
        );
        orchestrator.start_node(node, inputs).await?;

        let output_complete_event = stream.next().await.unwrap();
        dbg!(&output_complete_event);
        let output_complete_outputs = output_complete_event.outputs().unwrap();
        assert_registry_contains_values(
            &registry,
            &orchestrator.default_storage_name,
            &output_complete_outputs,
            json!({"value": 4}),
        );

        Ok(())
    }
}
