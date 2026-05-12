/*!
This module defines the [Orchestrator] struct that combines multiple [Executor][crate::executor::Executor]
and [`AssetStorage`][crate::asset_storage::AssetStorage] implementations to drive Workflow execution and return a stream
of [Event]s with updates about each node in the Workflow.
*/
use std::{
    collections::HashMap,
    sync::{Arc, LazyLock, Mutex},
};

use futures::{
    StreamExt,
    channel::mpsc,
    stream::{BoxStream, select_all},
};
use miette::{Context, IntoDiagnostic, miette};
use portgraph::NodeIndex;
use serde_json::Value;

use crate::{
    asset_storage::{
        AssetStorageRegistry,
        interface::{AssetKey, AssetSpec},
        transfer_assets,
    },
    event::{Event, send_complete},
    executor::{ExecutorRegistry, HPCExecutor, hpc::{HPCResourceSpec, HPCEnvironmentSpec}, interface::TaskPlan},
    graph::{NodeDefinition, WorkflowGraph},
};

static EMPTY_INPUTS: LazyLock<HashMap<String, AssetSpec>> = LazyLock::new(HashMap::new);

/// [Action] is a placeholder enum for operation the Executor can perform
/// when interpreting a Workflow graph.
#[derive(Debug, PartialEq, Eq)]
pub enum Action {
    /// An operation performed by a Worker.
    PerformTask {
        /// The name of the Worker to call.
        worker_name: String,
        /// The name of the Task to call.
        task_name: String,
        resources: HashMap<String, Value>,
        environment: HashMap<String, Value>,
    },
    /// An input to the Workflow.
    LoadInput {
        /// The name of the Workflow input to capture.
        name: String,
    },
    /// The output of the Workflow
    NotifyOutput {},
    /// A constant Value in the Workflow.
    LoadConst {
        /// The constant value to return.
        value: Value,
    },
}

#[allow(dead_code)]
#[derive(Debug)]
struct ExecutionContext<'a> {
    subworkflow_context: &'a [NodeIndex],
    graph_inputs: &'a HashMap<String, AssetSpec>,
    node_outputs: &'a HashMap<Vec<NodeIndex>, HashMap<String, AssetSpec>>,
}

/// [Orchestrator] manages the Workflow execution by dispatching [Node]s to the correct
/// [Executor][crate::executor::Executor] as well as managing a shared [`AssetStorageRegistry`]
/// for the Workflow.
pub struct Orchestrator {
    event_sender: mpsc::Sender<Event>,
    event_receiver: Mutex<Option<mpsc::Receiver<Event>>>,

    default_executor_name: String,
    executor_registry: ExecutorRegistry,

    default_storage_name: String,
    asset_storage_registry: AssetStorageRegistry,
}

impl Orchestrator {
    /// Try to create a new [Orchestrator] with an [`AssetStorageRegistry`] and an
    /// [`ExecutorRegistry`], as well as default options for the [`AssetStorage`][crate::asset_storage::AssetStorage]
    /// and [Executor][crate::executor::Executor] to use for each registry unless specified otherwise
    /// in the Workflow definition.
    ///
    /// # Errors
    ///
    /// This function will return Err if the specified `default_storage_name` does not exist
    /// inside the [`AssetStorageRegistry`] or if the specified `default_executor_name` does
    /// not exist inside the [`ExecutorRegistry`].
    pub fn try_new(
        asset_storage_registry: &AssetStorageRegistry,
        executor_registry: &ExecutorRegistry,
        default_storage_name: &str,
        default_executor_name: &str,
    ) -> miette::Result<Self> {
        let (sender, receiver) = mpsc::channel(128);

        let asset_storage_registry_lock = asset_storage_registry
            .read()
            .map_err(|err| miette!("Failed to lock AssetStorageRegistry for reading: {err}"))?;
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

    #[allow(dead_code)]
    fn build_actions<'a>(
        &self,
        context: &'a ExecutionContext<'a>,
        workflow_graph: &'a WorkflowGraph,
    ) -> impl Iterator<Item = (Action, HashMap<String, AssetSpec>)> {
        workflow_graph
            .toposort_filtered_from_output_node(|n| {
                let mut subworkflow_path = context.subworkflow_context.to_vec();
                subworkflow_path.push(n);
                !context.node_outputs.contains_key(&subworkflow_path)
            })
            .filter(|n| {
                workflow_graph.all_inputs(*n, |incoming| {
                    let mut subworkflow_path = context.subworkflow_context.to_vec();
                    subworkflow_path.push(incoming);
                    context.node_outputs.contains_key(&subworkflow_path)
                })
            })
            .flat_map(move |n| {
                let definition = workflow_graph.node_definition(n).unwrap();
                match definition {
                    NodeDefinition::Input { name } => {
                        vec![(
                            Action::LoadInput { name: name.clone() },
                            context.graph_inputs.clone(),
                        )]
                    }
                    NodeDefinition::Const { value } => vec![(
                        Action::LoadConst {
                            value: value.clone(),
                        },
                        EMPTY_INPUTS.clone(),
                    )],
                    NodeDefinition::Output {} => {
                        let inputs = collect_inputs(context, workflow_graph, n).unwrap();
                        vec![(Action::NotifyOutput {}, inputs)]
                    }
                    NodeDefinition::Task {
                        worker_name,
                        task_name,
                    } => {
                        let inputs = collect_inputs(context, workflow_graph, n).unwrap();
                        vec![(
                            Action::PerformTask {
                                worker_name: worker_name.clone(),
                                task_name: task_name.clone(),
                                resources: HashMap::new(),
                                environment: HashMap::new(),
                            },
                            inputs,
                        )]
                    }
                    NodeDefinition::Eval {} => {
                        let inputs = collect_inputs(context, workflow_graph, n).unwrap();
                        let graph_asset_spec = inputs.get("graph").unwrap();

                        let asset_storage = self.asset_storage_registry.read().unwrap();
                        let subgraph_storage =
                            asset_storage.get(&graph_asset_spec.storage_name).unwrap();
                        let subgraph_bytes =
                            subgraph_storage.load(&graph_asset_spec.asset_key).unwrap();
                        let subgraph: WorkflowGraph =
                            serde_json::from_slice(&subgraph_bytes).unwrap();

                        let mut subworkflow_context = context.subworkflow_context.to_vec();
                        subworkflow_context.push(n);

                        self.build_actions(
                            &ExecutionContext {
                                subworkflow_context: &subworkflow_context,
                                graph_inputs: &inputs,
                                node_outputs: context.node_outputs,
                            },
                            &subgraph,
                        )
                        .collect()
                    }
                }
            })
    }

    /// Perform a series of actions, dispatching to [`Executor`]s when necessary.
    ///
    /// # Errors
    ///
    /// Will return Err if a Node cannot be run or dispatched.
    pub async fn perform_actions(
        &self,
        actions: impl IntoIterator<Item = (Action, HashMap<String, AssetSpec>)>,
    ) -> miette::Result<()> {
        // Build a list of tasks to dispatch to Executors and immediately
        // process everything else.

        let mut exec_plans: HashMap<String, Vec<TaskPlan>> = HashMap::new();
        for exec in self.executor_registry.keys() {
            exec_plans.insert(exec.clone(), Vec::new());
        }

        //let mut task_plans = Vec::new();
        for (action, inputs) in actions {
            match action {
                Action::PerformTask {
                    worker_name,
                    task_name,
                    resources,
                    environment,
                } => {
                    let executor = self.orchestrate(&resources, &environment);
                    exec_plans.get_mut(&executor).unwrap().push(TaskPlan {
                        worker_name,
                        task_name,
                        inputs: inputs.clone(),
                        output_storage_name: Some(self.default_storage_name.clone()),
                        resources,
                        environment,
                        outputs: std::collections::HashSet::from_iter(vec!["value".to_string()]),
                        ..Default::default()
                    })
                //     task_plans.push(TaskPlan {
                //     worker_name,
                //     task_name,
                //     inputs: inputs.clone(),
                //     output_storage_name: Some(self.default_storage_name.clone()),
                //     resources,
                //     environment,
                //     ..Default::default()
                // })
                },
                Action::LoadInput { name } => {
                    // Assume the inputs are the inputs to the graph.
                    //
                    // Just pull out the named input and assign it to the "value" output.
                    let value_spec = inputs
                        .get(&name)
                        .ok_or(miette!("Missing input: {name}"))
                        .wrap_err("Could not run Input Node")?;
                    let mut outputs = HashMap::new();
                    outputs.insert("value".to_string(), value_spec.clone());

                    // Move the values if needed.
                    outputs = transfer_assets(
                        &self.asset_storage_registry,
                        &self.default_storage_name,
                        &outputs,
                    )?;

                    let mut event_sender = self.event_sender.clone();
                    send_complete(&mut event_sender, 0, outputs).await?;
                }
                Action::NotifyOutput {} => {
                    // Notify that the outputs are ready
                    let mut event_sender = self.event_sender.clone();

                    // Move the values if needed.
                    let outputs = transfer_assets(
                        &self.asset_storage_registry,
                        &self.default_storage_name,
                        &inputs,
                    )
                    .wrap_err("Could not run Output Node")?;
                    send_complete(&mut event_sender, 0, outputs).await?;
                }
                Action::LoadConst { value } => {
                    // Load the constant value into the correct storage.
                    //
                    // `outputs` is explicitly scoped to manage the lifetime of the
                    // `asset_storage_registry_lock` to avoid holding it across an `await`.
                    let outputs = {
                        let asset_storage_registry_lock =
                            self.asset_storage_registry.read().map_err(|err| {
                                miette!("Failed to lock AssetStorageRegistry for reading: {err}")
                            })?;
                        let default_storage_name = &self.default_storage_name;
                        let asset_storage = asset_storage_registry_lock
                    .get(default_storage_name)
                    .ok_or_else(|| miette!("Could not find a storage with name '{default_storage_name}' in AssetStorageRegistry"))?;
                        let asset_key = AssetKey::new();
                        asset_storage
                            .save(&asset_key, serde_json::to_vec(&value).into_diagnostic()?)?;

                        let mut outputs = HashMap::new();
                        outputs.insert(
                            "value".to_string(),
                            AssetSpec {
                                kind: asset_storage.kind(),
                                storage_name: self.default_storage_name.clone(),
                                asset_key,
                            },
                        );

                        outputs
                    };

                    let mut event_sender = self.event_sender.clone();
                    send_complete(&mut event_sender, 0, outputs).await?;
                }
            }
        }

        // let default_executor_name = &self.default_executor_name;
        // let executor = self
        //     .executor_registry
        //     .get(default_executor_name)
        //     .ok_or_else(|| miette!("Could not find a storage with name '{default_executor_name}' in ExecutorRegistry")).wrap_err("Could not run Task Nodes")?;
        for (executor_name, task_plans) in exec_plans {
            if task_plans.is_empty() {
                continue;
            }
            let executor = self
                .executor_registry
                .get(&executor_name)
                .ok_or_else(|| miette!("Could not find an executor with name '{executor_name}' in ExecutorRegistry"))
                .wrap_err("Could not run Task Nodes")?;
        
            executor
                .execute(task_plans)
                .await
                .wrap_err("Could not run Task Nodes")?;
        }
        Ok(())
    }

    fn orchestrate(&self, resources: &HashMap<String, Value>, environment: &HashMap<String, Value>,) -> String {
        for (executor_name, executor) in self.executor_registry.iter() {
            if let Some(hpc_executor) = executor.as_any().downcast_ref::<HPCExecutor>() {
                let hpc_resources = HPCResourceSpec::new(
                    resources.get("nodes").and_then(|v| v.as_u64()).unwrap_or(1) as u32,
                    resources.get("cores_per_node").and_then(|v| v.as_u64()).unwrap_or(1) as u32,
                    resources.get("memory_per_node_gb").and_then(|v| v.as_u64()).unwrap_or(1) as u32,
                    resources.get("gpus_per_node").and_then(|v| v.as_u64()).unwrap_or(0) as u32,
                );
                let hpc_environment = HPCEnvironmentSpec::new(
                    environment.get("mpi_available").and_then(|v| v.as_bool()).unwrap_or(false),
                );
                if hpc_executor.max_resources.satisfies(&hpc_resources) && hpc_executor.environment.satisfies(&hpc_environment) {
                    return executor_name.clone();
                }
            }
        }
        self.default_executor_name.clone()
    }

    /// Listen to a combined stream events from the Orchestrator and the
    /// Executors in the registry.
    ///
    /// This method should only be called once and the stream will exist
    /// for the duration of the Workflow execution.
    ///
    /// # Errors
    ///
    /// Will return Err if the method has already been called.
    pub fn listen(&self) -> miette::Result<BoxStream<'_, Event>> {
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
            .collect::<miette::Result<Vec<BoxStream<Event>>>>()?;

        streams.push(orchestrator_events.boxed());
        Ok(select_all(streams).boxed())
    }
}

fn collect_inputs(
    context: &ExecutionContext<'_>,
    workflow_graph: &WorkflowGraph,
    n: NodeIndex,
) -> miette::Result<HashMap<String, AssetSpec>> {
    let inputs: HashMap<String, AssetSpec> = workflow_graph
        .input_links(n)
        .map(|(i, o)| {
            let input_name = workflow_graph.get_port_name(&i)?;
            let output_name = workflow_graph.get_port_name(&o)?;
            let linked_node = workflow_graph.port_node(o)?;
            let output_asset_spec = context
                .node_outputs
                .get(&vec![linked_node])
                .ok_or_else(|| miette!("Could not find node outputs for node: {linked_node:?}"))?
                .get(output_name)
                .ok_or_else(|| miette!("Could not get node output for node: {linked_node:?} and port name: {output_name}"))?;
            Ok((input_name.clone(), output_asset_spec.clone()))
        })
        .collect::<miette::Result<_>>()
        .wrap_err(miette!("Could not collect inputs for node with id: {n:?}"))?;
    Ok(inputs)
}

#[cfg(test)]
mod tests {
    use futures::StreamExt;
    use rstest::rstest;
    use serde_json::json;

    use crate::{
        asset_storage::{assert_registry_contains_values, test_storage_registry, FileAssetStorage},
        event::Status,
        executor::{
            inmemory::InMemoryExecutor, interface::Executor, subprocess::SubprocessExecutor,
            hpc::HPCExecutor, hpc::HPCResourceSpec, hpc::HPCEnvironmentSpec,
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

        let environment = HPCEnvironmentSpec::new(true);
        executor_registry.insert(
            "large".to_string(),
            Box::new(HPCExecutor::try_new(asset_storage_registry, "checkpoints", "checkpoints", HPCResourceSpec::new(2, 1, 4, 1), environment.clone()).unwrap()),
        );
        executor_registry.insert(
            "small".to_string(),
            Box::new(HPCExecutor::try_new(asset_storage_registry, "checkpoints", "checkpoints", HPCResourceSpec::new(1, 1, 2, 1), environment).unwrap()),
        );

        Arc::new(executor_registry)
    }

    // Test that we can run a task node that dispatches to an executor.
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

        let node = Action::PerformTask {
            worker_name: "builtin".to_string(),
            task_name: "iadd".to_string(),
            resources: HashMap::new(),
            environment: HashMap::new(),
        };
        let inputs = input_sets[0].clone();
        orchestrator.perform_actions([(node, inputs)]).await?;

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

    // Test that we can run an input node that copies a value into storage.
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

        let node = Action::LoadInput {
            name: "a".to_string(),
        };
        let inputs = input_sets[0].clone();
        orchestrator.perform_actions([(node, inputs)]).await?;

        let node = Action::LoadInput {
            name: "b".to_string(),
        };
        let inputs = input_sets[0].clone();
        orchestrator.perform_actions([(node, inputs)]).await?;

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

    // Test that we can run an output node that emits a complete event with outputs.
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

        let node = Action::NotifyOutput {};
        let inputs = input_sets[0].clone();
        orchestrator.perform_actions([(node, inputs)]).await?;

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

    // Test that we can run a const node that copies a value into storage.
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

        let node = Action::LoadConst {
            value: "hello there".into(),
        };
        let inputs = HashMap::new();
        orchestrator.perform_actions([(node, inputs)]).await?;

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

    // Test that we can run a series of nodes one at a time
    // in a way that would be typical for a simple workflow.
    #[rstest]
    #[case("memory")]
    #[case("file")]
    #[tokio::test]
    async fn perform_simple_workflow(#[case] default_storage_name: &str) -> miette::Result<()> {
        let (registry, input_sets, _dir) = test_storage_registry(vec![json!({"a": 1})], vec![]);
        let executor_registry = test_executor_registry(&registry);
        let orchestrator = Orchestrator::try_new(
            &registry,
            &executor_registry,
            default_storage_name,
            "memory",
        )?;
        let mut stream = orchestrator.listen()?;

        let node = Action::LoadInput {
            name: "a".to_string(),
        };
        let inputs = input_sets[0].clone();
        orchestrator.perform_actions([(node, inputs)]).await?;

        let input_complete_event = stream.next().await.unwrap();
        let mut input_complete_outputs = input_complete_event.outputs().unwrap();
        assert_registry_contains_values(
            &registry,
            &orchestrator.default_storage_name,
            &input_complete_outputs,
            json!({"value": 1}),
        );

        let node = Action::LoadConst { value: 3.into() };
        let inputs = HashMap::new();
        orchestrator.perform_actions([(node, inputs)]).await?;

        let const_complete_event = stream.next().await.unwrap();
        let mut const_complete_outputs = const_complete_event.outputs().unwrap();
        assert_registry_contains_values(
            &registry,
            &orchestrator.default_storage_name,
            &const_complete_outputs,
            json!({"value": 3}),
        );

        let node = Action::PerformTask {
            worker_name: "builtin".to_string(),
            task_name: "iadd".to_string(),
            resources: HashMap::new(),
            environment: HashMap::new(),
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
        orchestrator.perform_actions([(node, inputs)]).await?;

        let task_running_event = stream.next().await.unwrap();
        assert_eq!(task_running_event.status, Status::Running);
        let task_complete_event = stream.next().await.unwrap();
        let mut task_complete_outputs = task_complete_event.outputs().unwrap();
        assert_registry_contains_values(
            &registry,
            &orchestrator.default_storage_name,
            &task_complete_outputs,
            json!({"value": 4}),
        );

        let node = Action::NotifyOutput {};
        let mut inputs = HashMap::new();
        inputs.insert(
            "value".to_string(),
            task_complete_outputs.remove("value").unwrap(),
        );
        orchestrator.perform_actions([(node, inputs)]).await?;

        let output_complete_event = stream.next().await.unwrap();
        let output_complete_outputs = output_complete_event.outputs().unwrap();
        assert_registry_contains_values(
            &registry,
            &orchestrator.default_storage_name,
            &output_complete_outputs,
            json!({"value": 4}),
        );

        Ok(())
    }

    // Test that we can plan a workflow with two input nodes.
    #[rstest]
    #[case("memory")]
    #[case("file")]
    #[tokio::test]
    async fn plan_two_input_workflow(#[case] default_storage_name: &str) -> miette::Result<()> {
        let (registry, input_sets, _dir) =
            test_storage_registry(vec![json!({"a": 1, "b": 4})], vec![]);
        let executor_registry = test_executor_registry(&registry);
        let orchestrator = Orchestrator::try_new(
            &registry,
            &executor_registry,
            default_storage_name,
            "memory",
        )?;

        let mut workflow_graph = WorkflowGraph::new(["out1".to_string(), "out2".to_string()]);
        let input1_idx = workflow_graph.add_node(
            NodeDefinition::Input {
                name: "a".to_string(),
            },
            [],
            ["value".to_string()],
        );
        workflow_graph
            .link_nodes_by_port_name(&input1_idx, "value", &workflow_graph.output_idx(), "out1")
            .unwrap();
        let input2_idx = workflow_graph.add_node(
            NodeDefinition::Input {
                name: "b".to_string(),
            },
            [],
            ["value".to_string()],
        );
        workflow_graph
            .link_nodes_by_port_name(&input2_idx, "value", &workflow_graph.output_idx(), "out2")
            .unwrap();

        let inputs = input_sets[0].clone();
        let context = &ExecutionContext {
            subworkflow_context: &[],
            graph_inputs: &inputs.clone(),
            node_outputs: &HashMap::new(),
        };
        let actions = orchestrator.build_actions(context, &workflow_graph);
        let actions = actions.collect::<Vec<_>>();

        assert_eq!(actions.len(), 2);
        assert_eq!(
            actions[0].0,
            Action::LoadInput {
                name: "a".to_string()
            }
        );
        assert_eq!(
            actions[1].0,
            Action::LoadInput {
                name: "b".to_string()
            }
        );

        Ok(())
    }

    // Test that we can plan the first actions of a workflow and run them.
    #[rstest]
    #[case("memory")]
    #[case("file")]
    #[tokio::test]
    async fn plan_and_run_simple_io_workflow(
        #[case] default_storage_name: &str,
    ) -> miette::Result<()> {
        let (registry, input_sets, _dir) = test_storage_registry(vec![json!({"a": 1})], vec![]);
        let executor_registry = test_executor_registry(&registry);
        let orchestrator = Orchestrator::try_new(
            &registry,
            &executor_registry,
            default_storage_name,
            "memory",
        )?;
        let mut stream = orchestrator.listen()?;

        let mut workflow_graph = WorkflowGraph::new(["out".to_string()]);
        let input_idx = workflow_graph.add_node(
            NodeDefinition::Input {
                name: "a".to_string(),
            },
            [],
            ["value".to_string()],
        );

        workflow_graph
            .link_nodes_by_port_name(&input_idx, "value", &workflow_graph.output_idx(), "out")
            .unwrap();

        let inputs = input_sets[0].clone();
        let context = &ExecutionContext {
            subworkflow_context: &[],
            graph_inputs: &inputs.clone(),
            node_outputs: &HashMap::new(),
        };
        let actions = orchestrator.build_actions(context, &workflow_graph);
        let actions = actions.collect::<Vec<_>>();

        assert_eq!(actions.len(), 1);
        assert_eq!(
            actions[0].0,
            Action::LoadInput {
                name: "a".to_string()
            }
        );

        orchestrator.perform_actions(actions).await?;
        let input_complete_event = stream.next().await.unwrap();
        let input_complete_outputs = input_complete_event.outputs().unwrap();
        assert_registry_contains_values(
            &registry,
            &orchestrator.default_storage_name,
            &input_complete_outputs,
            json!({"value": 1}),
        );

        let mut node_outputs = HashMap::new();
        node_outputs.insert(vec![input_idx], input_complete_outputs);
        let context = &ExecutionContext {
            subworkflow_context: &[],
            graph_inputs: &inputs.clone(),
            node_outputs: &node_outputs,
        };

        let actions = orchestrator.build_actions(context, &workflow_graph);
        let actions = actions.collect::<Vec<_>>();

        assert_eq!(actions.len(), 1);
        assert_eq!(actions[0].0, Action::NotifyOutput {});

        orchestrator.perform_actions(actions).await?;
        let output_complete_event = stream.next().await.unwrap();
        let output_complete_outputs = output_complete_event.outputs().unwrap();
        assert_registry_contains_values(
            &registry,
            &orchestrator.default_storage_name,
            &output_complete_outputs,
            json!({"out": 1}),
        );

        Ok(())
    }

    // Test that we can plan the first actions of a workflow and run them.
    #[rstest]
    #[case("memory")]
    #[case("file")]
    #[tokio::test]
    async fn plan_and_run_simple_eval_workflow(
        #[case] default_storage_name: &str,
    ) -> miette::Result<()> {
        let mut subworkflow_graph = WorkflowGraph::new(["out".to_string()]);
        let inner_a_input_idx = subworkflow_graph.add_node(
            NodeDefinition::Input {
                name: "a".to_string(),
            },
            [],
            ["value".to_string()],
        );

        subworkflow_graph
            .link_nodes_by_port_name(
                &inner_a_input_idx,
                "value",
                &subworkflow_graph.output_idx(),
                "out",
            )
            .unwrap();

        let (registry, input_sets, _dir) = test_storage_registry(
            vec![json!({"a": 1, "subworkflow": subworkflow_graph})],
            vec![],
        );
        let executor_registry = test_executor_registry(&registry);
        let orchestrator = Orchestrator::try_new(
            &registry,
            &executor_registry,
            default_storage_name,
            "memory",
        )?;
        let mut stream = orchestrator.listen()?;

        let mut workflow_graph = WorkflowGraph::new(["out".to_string()]);
        let a_input_idx = workflow_graph.add_node(
            NodeDefinition::Input {
                name: "a".to_string(),
            },
            [],
            ["value".to_string()],
        );
        let subworkflow_input_idx = workflow_graph.add_node(
            NodeDefinition::Input {
                name: "subworkflow".to_string(),
            },
            [],
            ["value".to_string()],
        );
        let eval_idx = workflow_graph.add_node(
            NodeDefinition::Eval {},
            ["graph".to_string(), "a".to_string()],
            ["out".to_string()],
        );

        workflow_graph
            .link_nodes_by_port_name(&a_input_idx, "value", &eval_idx, "a")
            .unwrap();
        workflow_graph
            .link_nodes_by_port_name(&subworkflow_input_idx, "value", &eval_idx, "graph")
            .unwrap();
        workflow_graph
            .link_nodes_by_port_name(&eval_idx, "out", &workflow_graph.output_idx(), "out")
            .unwrap();

        let inputs = input_sets[0].clone();
        let context = &ExecutionContext {
            subworkflow_context: &[],
            graph_inputs: &inputs,
            node_outputs: &HashMap::new(),
        };
        let actions = orchestrator.build_actions(context, &workflow_graph);
        let actions = actions.collect::<Vec<_>>();

        assert_eq!(actions.len(), 2);
        assert_eq!(
            actions[0].0,
            Action::LoadInput {
                name: "subworkflow".to_string()
            }
        );
        assert_eq!(
            actions[1].0,
            Action::LoadInput {
                name: "a".to_string()
            }
        );

        orchestrator.perform_actions(actions).await?;
        let subworkflow_input_complete_event = stream.next().await.unwrap();
        let subworkflow_input_complete_outputs =
            subworkflow_input_complete_event.outputs().unwrap();
        assert_registry_contains_values(
            &registry,
            &orchestrator.default_storage_name,
            &subworkflow_input_complete_outputs,
            json!({"value": subworkflow_graph}),
        );

        let a_input_complete_event = stream.next().await.unwrap();
        let a_input_complete_outputs = a_input_complete_event.outputs().unwrap();
        assert_registry_contains_values(
            &registry,
            &orchestrator.default_storage_name,
            &a_input_complete_outputs,
            json!({"value": 1}),
        );

        let mut node_outputs = HashMap::new();
        node_outputs.insert(
            vec![subworkflow_input_idx],
            subworkflow_input_complete_outputs,
        );
        node_outputs.insert(vec![a_input_idx], a_input_complete_outputs);
        let context = &ExecutionContext {
            subworkflow_context: &[],
            graph_inputs: &inputs,
            node_outputs: &node_outputs,
        };

        let actions = orchestrator.build_actions(context, &workflow_graph);
        let actions = actions.collect::<Vec<_>>();

        assert_eq!(actions.len(), 1);
        assert_eq!(
            actions[0].0,
            Action::LoadInput {
                name: "a".to_string()
            }
        );

        orchestrator.perform_actions(actions).await?;
        let inner_a_input_complete_event = stream.next().await.unwrap();
        let inner_a_input_complete_outputs = inner_a_input_complete_event.outputs().unwrap();
        assert_registry_contains_values(
            &registry,
            &orchestrator.default_storage_name,
            &inner_a_input_complete_outputs,
            json!({"value": 1}),
        );

        node_outputs.insert(
            vec![eval_idx, inner_a_input_idx],
            inner_a_input_complete_outputs,
        );
        let context = &ExecutionContext {
            subworkflow_context: &[],
            graph_inputs: &inputs,
            node_outputs: &node_outputs,
        };

        let actions = orchestrator.build_actions(context, &workflow_graph);
        let actions = actions.collect::<Vec<_>>();

        orchestrator.perform_actions(actions).await?;
        let inner_output_complete_event = stream.next().await.unwrap();
        let inner_output_complete_outputs = inner_output_complete_event.outputs().unwrap();
        assert_registry_contains_values(
            &registry,
            &orchestrator.default_storage_name,
            &inner_output_complete_outputs,
            json!({"out": 1}),
        );

        // TODO: Does this represent what the updater will actually do?
        node_outputs.insert(
            vec![eval_idx, subworkflow_graph.output_idx()],
            inner_output_complete_outputs.clone(),
        );
        node_outputs.insert(vec![eval_idx], inner_output_complete_outputs);
        let context = &ExecutionContext {
            subworkflow_context: &[],
            graph_inputs: &inputs,
            node_outputs: &node_outputs,
        };

        let actions = orchestrator.build_actions(context, &workflow_graph);
        let actions = actions.collect::<Vec<_>>();

        orchestrator.perform_actions(actions).await?;
        let output_complete_event = stream.next().await.unwrap();
        let output_complete_outputs = output_complete_event.outputs().unwrap();
        assert_registry_contains_values(
            &registry,
            &orchestrator.default_storage_name,
            &output_complete_outputs,
            json!({"out": 1}),
        );

        Ok(())
    }
   
    #[rstest]
    #[tokio::test]
    async fn do_resource_orchestration() -> miette::Result<()> {
        // Setup Orchestrator and registry
        let file_storage = FileAssetStorage::new(std::path::Path::new("/Users/philipp.seitz/.tierkreis/checkpoints/00000000-0000-0000-0000-000000000016/"));
        let (registry, input_sets, _dir) = test_storage_registry(vec![json!({"value": "Test"})], vec![]);
        registry.write().unwrap().insert("checkpoints".to_string(), Box::new(file_storage));
        let executor_registry = test_executor_registry(&registry);


        let executors = Arc::new(executor_registry);
        let orchestrator = Orchestrator::try_new(
            &registry,
            &executors,
            "memory",
            "memory",
        )?;
        let stream = orchestrator.listen()?;

        let node = Action::PerformTask {
            worker_name: "mpi_worker".to_string(),
            task_name: "mpi_rank_info_with_input".to_string(),
            resources: HashMap::from([("nodes".to_string(), json!(2))]),
            environment: HashMap::from([("use_mpi".to_string(), json!(true))]),
        };
        let inputs = input_sets[0].clone();
        orchestrator.perform_actions([(node, inputs)]).await?;
        let events = stream.take(2).collect::<Vec<_>>().await;
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].status, Status::Running);
        dbg!(events.clone());
        assert_registry_contains_values(
            &registry,
            "memory",
            &events[1].clone().outputs().unwrap_or_default(),
            json!({"value": "Rank 0 out of 2 on c1 with value Test.\nRank 1 out of 2 on c2 with value Test."}),
        );


        Ok(())
    }
}
 