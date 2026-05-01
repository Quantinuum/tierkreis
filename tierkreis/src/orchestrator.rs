/*!
This module defines the [Orchestrator] struct that combines multiple [Executor][crate::executor::Executor]
and [`AssetStorage`][crate::asset_storage::AssetStorage] implementations to drive Workflow execution and return a stream
of [Event]s with updates about each node in the Workflow.
*/
use std::{
    collections::{HashMap, HashSet},
    sync::{Arc, LazyLock, Mutex},
};

use futures::{
    Stream, StreamExt, TryFutureExt,
    channel::mpsc,
    future,
    stream::{self, BoxStream, select_all},
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
    executor::{ExecutorRegistry, interface::TaskPlan},
    graph::{LegacyWorkflowGraph, NodeDefinition, WorkflowGraph},
    location::Location,
    state::WorkflowState,
};

static EMPTY_INPUTS: LazyLock<HashMap<String, AssetSpec>> = LazyLock::new(HashMap::new);

/// `Action` describes an operation the Orchestrator should perform.
#[derive(Debug, Clone, PartialEq)]
pub struct Action {
    /// The node location in the graph where orchestration should occur.
    pub loc: Location,
    /// The kind of action to perform.
    pub kind: ActionKind,
    /// The inputs from edges in the graph if relevant to the action.
    pub inputs: HashMap<String, AssetSpec>,
}

/// [`ActionKind`] is a placeholder enum for operation the [`Orchestrator`] can perform
/// when interpreting a [`WorkflowGraph`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ActionKind {
    /// An operation performed by a Worker.
    PerformTask {
        /// The name of the Worker to call.
        worker_name: String,
        /// The name of the Task to call.
        task_name: String,
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
#[derive(Debug, Clone)]
struct ExecutionContext {
    subworkflow_context: Location,
    graph_inputs: HashMap<String, AssetSpec>,
    workflow_state: Arc<dyn WorkflowState>,
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
    fn build_actions(
        &self,
        context: ExecutionContext,
        workflow_graph: WorkflowGraph,
    ) -> BoxStream<'_, miette::Result<Action>> {
        let fut = async move {
            if context
                .workflow_state
                .is_scheduled(&Location::from_node_index_iter([
                    workflow_graph.output_idx()
                ]))
                .await?
            {
                // Output is already scheduled, no actions to perform.
                return Ok(stream::empty().boxed());
            }

            let ready_nodes = prepare_ready_nodes(&context, &workflow_graph).await?;

            let parent_location = context.subworkflow_context.clone();
            let graph_inputs = context.graph_inputs.clone();

            Ok(stream::iter(ready_nodes)
                .flat_map_unordered(None, move |n| {
                    let Some(definition) = workflow_graph.node_definition(n) else {
                        return stream_error(miette!("Could not find node definition"));
                    };

                    let loc = parent_location.with_node(n);
                    match definition {
                        NodeDefinition::Input { name } => stream_ready_action(Action {
                            loc,
                            kind: ActionKind::LoadInput {
                                name: name.to_owned(),
                            },
                            inputs: graph_inputs.clone(),
                        }),
                        NodeDefinition::Const { value } => stream_ready_action(Action {
                            loc,
                            kind: ActionKind::LoadConst {
                                value: value.clone(),
                            },
                            inputs: EMPTY_INPUTS.clone(),
                        }),
                        NodeDefinition::Output {} => {
                            build_output_action(&context, &workflow_graph, n, loc)
                        }
                        NodeDefinition::Task {
                            worker_name,
                            task_name,
                        } => build_task_action(
                            &context,
                            &workflow_graph,
                            n,
                            loc,
                            worker_name,
                            task_name,
                        ),
                        NodeDefinition::Eval {} => {
                            self.build_subgraph_actions(&context, &workflow_graph, n, loc)
                        }
                        _ => unimplemented!(),
                    }
                })
                .boxed())
        };

        fut.try_flatten_stream().boxed()
    }

    fn build_subgraph_actions(
        &self,
        context: &ExecutionContext,
        workflow_graph: &WorkflowGraph,
        n: NodeIndex,
        loc: Location,
    ) -> std::pin::Pin<Box<dyn Stream<Item = Result<Action, miette::Error>> + Send + '_>> {
        let context = context.clone();
        let workflow_graph = workflow_graph.clone();
        async move {
            // TODO: we don't need to collect inputs other than the graph
            // itself if the graph has already started.
            let inputs = collect_inputs(&context, &workflow_graph, n).await?;

            let graph_asset_spec = inputs.get("graph").unwrap();

            let asset_storage = self.asset_storage_registry.read().unwrap();
            let subgraph_storage = asset_storage.get(&graph_asset_spec.storage_name).unwrap();
            let subgraph_bytes = subgraph_storage.load(&graph_asset_spec.asset_key)?;
            let subgraph_res: Result<WorkflowGraph, serde_json::Error> =
                serde_json::from_slice(&subgraph_bytes);

            let subgraph = match subgraph_res {
                Ok(subgraph) => subgraph,
                Err(_err) => {
                    // TODO: rich error message here
                    let legacy: LegacyWorkflowGraph =
                        serde_json::from_slice(&subgraph_bytes).into_diagnostic()?;
                    legacy.to_workflow_graph()?
                    _ => unimplemented!(),
                }
            };

            Ok(self.build_actions(
                ExecutionContext {
                    subworkflow_context: loc,
                    graph_inputs: inputs,
                    workflow_state: Arc::clone(&context.workflow_state),
                },
                subgraph,
            ))
        }
        .try_flatten_stream()
        .boxed()
    }

    /// Perform a series of actions, dispatching to [`Executor`]s when necessary.
    ///
    /// # Errors
    ///
    /// Will return Err if a Node cannot be run or dispatched.
    pub async fn perform_actions(
        &self,
        mut actions: impl Stream<Item = miette::Result<Action>> + Unpin,
    ) -> miette::Result<()> {
        // Build a list of tasks to dispatch to Executors and immediately
        // process everything else.
        let mut task_plans = Vec::new();
        while let Some(Action { loc, kind, inputs }) = actions.next().await.transpose()? {
            match kind {
                ActionKind::PerformTask {
                    worker_name,
                    task_name,
                } => task_plans.push(TaskPlan {
                    loc,
                    worker_name,
                    task_name,
                    inputs: inputs.clone(),
                    output_storage_name: Some(self.default_storage_name.clone()),
                    ..Default::default()
                }),
                ActionKind::LoadInput { name } => self.load_input(name, loc, &inputs).await?,
                ActionKind::NotifyOutput {} => self.notify_output(loc, inputs).await?,
                ActionKind::LoadConst { value } => self.load_const(value, loc).await?,
            }
        }

        let default_executor_name = &self.default_executor_name;
        let executor = self
            .executor_registry
            .get(default_executor_name)
            .ok_or_else(|| miette!("Could not find a storage with name '{default_executor_name}' in ExecutorRegistry")).wrap_err("Could not run Task Nodes")?;
        executor
            .execute(task_plans)
            .await
            .wrap_err("Could not run Task Nodes")?;

        Ok(())
    }

    async fn load_const(&self, value: Value, loc: Location) -> Result<(), miette::Error> {
        let outputs = {
            let asset_storage_registry_lock = self
                .asset_storage_registry
                .read()
                .map_err(|err| miette!("Failed to lock AssetStorageRegistry for reading: {err}"))?;
            let default_storage_name = &self.default_storage_name;
            let asset_storage = asset_storage_registry_lock
                .get(default_storage_name)
                .ok_or_else(|| miette!("Could not find a storage with name '{default_storage_name}' in AssetStorageRegistry"))?;

            let asset_key = AssetKey::new();
            asset_storage.save(&asset_key, serde_json::to_vec(&value).into_diagnostic()?)?;

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
        send_complete(&mut event_sender, loc, outputs).await?;
        Ok(())
    }

    async fn notify_output(
        &self,
        loc: Location,
        inputs: HashMap<String, AssetSpec>,
    ) -> Result<(), miette::Error> {
        let outputs = transfer_assets(
            &self.asset_storage_registry,
            &self.default_storage_name,
            &inputs,
        )
        .wrap_err("Could not run Output Node")?;

        let parent = loc.parent();
        let mut event_sender = self.event_sender.clone();
        send_complete(&mut event_sender, loc, outputs.clone()).await?;
        send_complete(&mut event_sender, parent, outputs).await?;
        Ok(())
    }

    async fn load_input(
        &self,
        name: String,
        loc: Location,
        inputs: &HashMap<String, AssetSpec>,
    ) -> Result<(), miette::Error> {
        let value_spec = inputs
            .get(&name)
            .ok_or(miette!("Missing input: {name}"))
            .wrap_err("Could not run Input Node")?;

        let mut outputs = HashMap::new();
        outputs.insert(name, value_spec.clone());
        outputs = transfer_assets(
            &self.asset_storage_registry,
            &self.default_storage_name,
            &outputs,
        )?;

        let mut event_sender = self.event_sender.clone();
        send_complete(&mut event_sender, loc, outputs).await?;
        Ok(())
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
    pub fn listen(&self) -> miette::Result<impl Stream<Item = Event> + use<>> {
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
        Ok(select_all(streams))
    }
}

fn stream_ready_action(
    action: Action,
) -> std::pin::Pin<Box<dyn Stream<Item = Result<Action, miette::Error>> + Send>> {
    stream::once(future::ok(action)).boxed()
}

fn stream_error(
    err: miette::Error,
) -> std::pin::Pin<Box<dyn Stream<Item = Result<Action, miette::Error>> + Send>> {
    stream::once(future::err(err)).boxed()
}

fn build_task_action(
    context: &ExecutionContext,
    workflow_graph: &WorkflowGraph,
    n: NodeIndex,
    loc: Location,
    worker_name: &str,
    task_name: &str,
) -> std::pin::Pin<Box<dyn Stream<Item = Result<Action, miette::Error>> + Send>> {
    stream::once({
        let context = context.clone();
        let workflow_graph = workflow_graph.clone();
        let worker_name = worker_name.to_owned();
        let task_name = task_name.to_owned();
        async move {
            let inputs = collect_inputs(&context, &workflow_graph, n).await?;

            Ok(Action {
                loc,
                kind: ActionKind::PerformTask {
                    worker_name,
                    task_name,
                },
                inputs,
            })
        }
    })
    .boxed()
}

fn build_output_action(
    context: &ExecutionContext,
    workflow_graph: &WorkflowGraph,
    n: NodeIndex,
    loc: Location,
) -> std::pin::Pin<Box<dyn Stream<Item = Result<Action, miette::Error>> + Send>> {
    stream::once({
        let context = context.clone();
        let workflow_graph = workflow_graph.clone();
        async move {
            let inputs = collect_inputs(&context, &workflow_graph, n).await?;

            Ok(Action {
                loc,
                kind: ActionKind::NotifyOutput {},
                inputs,
            })
        }
    })
    .boxed()
}

/// Find nodes which are ready for execution, mark them as scheduled then return them.
async fn prepare_ready_nodes(
    context: &ExecutionContext,
    workflow_graph: &WorkflowGraph,
) -> miette::Result<Vec<NodeIndex>> {
    // Find relevant nodes and check their state.
    let mut scheduled_nodes: HashSet<NodeIndex> = HashSet::new();
    let mut nodes_with_outputs: HashSet<NodeIndex> = HashSet::new();
    for node_id in workflow_graph.node_ids() {
        let location = context.subworkflow_context.with_node(node_id);
        if context.workflow_state.is_scheduled(&location).await? {
            scheduled_nodes.insert(node_id);

            if context.workflow_state.has_outputs(&location).await? {
                nodes_with_outputs.insert(node_id);
            }
        }
    }

    // Find nodes that are ready for scheduling.
    let ready_nodes: Vec<_> = workflow_graph
        .toposort_filtered_from_output_node(|n| {
            !scheduled_nodes.contains(&n)
                || matches!(
                    workflow_graph
                        .node_definition(n)
                        .expect("Node definition not found"),
                    NodeDefinition::Eval {}
                )
        })
        .filter(|n| {
            workflow_graph.all_inputs(*n, |incoming| nodes_with_outputs.contains(&incoming))
        })
        .collect();

    // Mark the ready nodes as scheduled.
    for ready_node in &ready_nodes {
        context
            .workflow_state
            .write(Event {
                loc: context.subworkflow_context.with_node(*ready_node),
                status: crate::event::Status::Scheduled {},
            })
            .await?;
    }

    Ok(ready_nodes)
}

async fn collect_inputs(
    context: &ExecutionContext,
    workflow_graph: &WorkflowGraph,
    n: NodeIndex,
) -> miette::Result<HashMap<String, AssetSpec>> {
    let inputs: HashMap<String, AssetSpec> = stream::iter(workflow_graph.input_links(n))
        .then(|(i, o)| async move {
            let input_name = workflow_graph.get_port_name(&i.into())?;
            let output_name = workflow_graph.get_port_name(&o.into())?;
            let linked_node = workflow_graph.port_node(o)?;
            let loc = context.subworkflow_context.with_node(linked_node);
            let node_state = context
                .workflow_state
                .read(&loc)
                .await
                .wrap_err(miette!("Could not find node outputs for location: {loc:?}"))?;
            let outputs = node_state
                .outputs
                .ok_or_else(|| miette!("Could not find node outputs for location: {loc:?}"))?;
            let output_asset_spec = outputs.get(output_name).ok_or_else(|| {
                miette!(
                    "Could not get node output for location: {loc:?} and port name: {output_name}"
                )
            })?;
            Ok((input_name.clone(), output_asset_spec.clone()))
        })
        .collect::<Vec<miette::Result<_, miette::Report>>>()
        .await
        .into_iter()
        .collect::<miette::Result<HashMap<_, _>>>()?;
    Ok(inputs)
}

#[cfg(test)]
mod tests {
    use futures::StreamExt;
    use rstest::rstest;
    use serde_json::json;

    use crate::{
        asset_storage::{assert_registry_contains_values, test_storage_registry},
        event::Status,
        executor::{
            inmemory::InMemoryExecutor, interface::Executor, subprocess::SubprocessExecutor,
        },
        graph::LegacyWorkflowGraph,
        state::{inmemory::InMemoryWorkflowState, interface::NodeState},
        updater::Updater,
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

        let kind = ActionKind::PerformTask {
            worker_name: "builtin".to_string(),
            task_name: "iadd".to_string(),
        };
        let inputs = input_sets[0].clone();
        orchestrator
            .perform_actions(stream::iter([Ok(Action {
                loc: Location::root(),
                kind,
                inputs,
            })]))
            .await?;

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

        let kind = ActionKind::LoadInput {
            name: "a".to_string(),
        };
        let inputs = input_sets[0].clone();
        orchestrator
            .perform_actions(stream::iter([Ok(Action {
                loc: Location::root(),
                kind,
                inputs,
            })]))
            .await?;

        let kind = ActionKind::LoadInput {
            name: "b".to_string(),
        };
        let inputs = input_sets[0].clone();
        orchestrator
            .perform_actions(stream::iter([Ok(Action {
                loc: Location::root(),
                kind,
                inputs,
            })]))
            .await?;

        let events = stream.take(2).collect::<Vec<_>>().await;
        assert_eq!(events.len(), 2);
        assert_registry_contains_values(
            &registry,
            &orchestrator.default_storage_name,
            &events[0].clone().outputs().unwrap(),
            json!({"a": 1}),
        );
        assert_registry_contains_values(
            &registry,
            &orchestrator.default_storage_name,
            &events[1].clone().outputs().unwrap(),
            json!({"b": 3}),
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

        let kind = ActionKind::NotifyOutput {};
        let inputs = input_sets[0].clone();
        orchestrator
            .perform_actions(stream::iter([Ok(Action {
                loc: Location::root(),
                kind,
                inputs,
            })]))
            .await?;

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

        let kind = ActionKind::LoadConst {
            value: "hello there".into(),
        };
        let inputs = HashMap::new();
        orchestrator
            .perform_actions(stream::iter([Ok(Action {
                loc: Location::root(),
                kind,
                inputs,
            })]))
            .await?;

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

        let kind = ActionKind::LoadInput {
            name: "a".to_string(),
        };
        let inputs = input_sets[0].clone();
        orchestrator
            .perform_actions(stream::iter([Ok(Action {
                loc: Location::root(),
                kind,
                inputs,
            })]))
            .await?;

        let input_complete_event = stream.next().await.unwrap();
        let mut input_complete_outputs = input_complete_event.outputs().unwrap();
        assert_registry_contains_values(
            &registry,
            &orchestrator.default_storage_name,
            &input_complete_outputs,
            json!({"a": 1}),
        );

        let kind = ActionKind::LoadConst { value: 3.into() };
        let inputs = HashMap::new();
        orchestrator
            .perform_actions(stream::iter([Ok(Action {
                loc: Location::root(),
                kind,
                inputs,
            })]))
            .await?;

        let const_complete_event = stream.next().await.unwrap();
        let mut const_complete_outputs = const_complete_event.outputs().unwrap();
        assert_registry_contains_values(
            &registry,
            &orchestrator.default_storage_name,
            &const_complete_outputs,
            json!({"value": 3}),
        );

        let kind = ActionKind::PerformTask {
            worker_name: "builtin".to_string(),
            task_name: "iadd".to_string(),
        };
        let mut inputs = HashMap::new();
        inputs.insert("a".to_string(), input_complete_outputs.remove("a").unwrap());
        inputs.insert(
            "b".to_string(),
            const_complete_outputs.remove("value").unwrap(),
        );
        orchestrator
            .perform_actions(stream::iter([Ok(Action {
                loc: Location::root(),
                kind,
                inputs,
            })]))
            .await?;

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

        let kind = ActionKind::NotifyOutput {};
        let mut inputs = HashMap::new();
        inputs.insert(
            "value".to_string(),
            task_complete_outputs.remove("value").unwrap(),
        );
        orchestrator
            .perform_actions(stream::iter([Ok(Action {
                loc: Location::root(),
                kind,
                inputs,
            })]))
            .await?;

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
            ["a".to_string()],
        );
        workflow_graph
            .link_nodes_by_port_name(&input1_idx, "a", &workflow_graph.output_idx(), "out1")
            .unwrap();
        let input2_idx = workflow_graph.add_node(
            NodeDefinition::Input {
                name: "b".to_string(),
            },
            [],
            ["b".to_string()],
        );
        workflow_graph
            .link_nodes_by_port_name(&input2_idx, "b", &workflow_graph.output_idx(), "out2")
            .unwrap();

        let (workflow_state, _state_events) = InMemoryWorkflowState::test();
        let workflow_state: Arc<dyn WorkflowState> = Arc::new(workflow_state);
        let inputs = input_sets[0].clone();
        let context = ExecutionContext {
            subworkflow_context: Location::root(),
            graph_inputs: inputs.clone(),
            workflow_state,
        };
        let actions = orchestrator.build_actions(context, workflow_graph);
        let actions = actions.collect::<Vec<_>>().await;

        assert_eq!(actions.len(), 2);
        let action0 = actions[0].as_ref().unwrap();
        assert_eq!(
            action0.kind,
            ActionKind::LoadInput {
                name: "a".to_string()
            }
        );
        let action1 = actions[1].as_ref().unwrap();
        assert_eq!(
            action1.kind,
            ActionKind::LoadInput {
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
            ["a".to_string()],
        );

        workflow_graph
            .link_nodes_by_port_name(&input_idx, "a", &workflow_graph.output_idx(), "out")
            .unwrap();

        let inputs = input_sets[0].clone();
        let (workflow_state, _state_events) = InMemoryWorkflowState::test();
        let workflow_state: Arc<dyn WorkflowState> = Arc::new(workflow_state);
        let context = ExecutionContext {
            subworkflow_context: Location::root(),
            graph_inputs: inputs.clone(),
            workflow_state: Arc::clone(&workflow_state),
        };
        let actions = orchestrator.build_actions(context, workflow_graph.clone());
        let actions = actions.collect::<Vec<_>>().await;

        assert_eq!(actions.len(), 1);
        let action0 = actions[0].as_ref().unwrap();
        assert_eq!(action0.loc, Location::from_node_index_iter([input_idx]));
        assert_eq!(
            action0.kind,
            ActionKind::LoadInput {
                name: "a".to_string()
            }
        );

        orchestrator.perform_actions(stream::iter(actions)).await?;
        let input_complete_event = stream.next().await.unwrap();
        let input_complete_outputs = input_complete_event.clone().outputs().unwrap();
        assert_registry_contains_values(
            &registry,
            &orchestrator.default_storage_name,
            &input_complete_outputs,
            json!({"a": 1}),
        );

        workflow_state.write(input_complete_event).await?;
        let context = ExecutionContext {
            subworkflow_context: Location::root(),
            graph_inputs: inputs.clone(),
            workflow_state: Arc::clone(&workflow_state),
        };
        let actions = orchestrator.build_actions(context, workflow_graph.clone());
        let actions = actions.collect::<Vec<_>>().await;

        assert_eq!(actions.len(), 1);
        let action0 = actions[0].as_ref().unwrap();
        assert_eq!(
            action0.loc,
            Location::from_node_index_iter([workflow_graph.output_idx()])
        );
        assert_eq!(action0.kind, ActionKind::NotifyOutput {});

        orchestrator.perform_actions(stream::iter(actions)).await?;
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
            ["a".to_string()],
        );

        subworkflow_graph
            .link_nodes_by_port_name(
                &inner_a_input_idx,
                "a",
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
            ["a".to_string()],
        );
        let subworkflow_input_idx = workflow_graph.add_node(
            NodeDefinition::Input {
                name: "subworkflow".to_string(),
            },
            [],
            ["subworkflow".to_string()],
        );
        let eval_idx = workflow_graph.add_node(
            NodeDefinition::Eval {},
            ["graph".to_string(), "a".to_string()],
            ["out".to_string()],
        );

        workflow_graph
            .link_nodes_by_port_name(&a_input_idx, "a", &eval_idx, "a")
            .unwrap();
        workflow_graph
            .link_nodes_by_port_name(&subworkflow_input_idx, "subworkflow", &eval_idx, "graph")
            .unwrap();
        workflow_graph
            .link_nodes_by_port_name(&eval_idx, "out", &workflow_graph.output_idx(), "out")
            .unwrap();

        let (workflow_state, _state_events) = InMemoryWorkflowState::test();
        let workflow_state: Arc<dyn WorkflowState> = Arc::new(workflow_state);
        let inputs = input_sets[0].clone();
        let context = ExecutionContext {
            subworkflow_context: Location::root(),
            graph_inputs: inputs.clone(),
            workflow_state: Arc::clone(&workflow_state),
        };
        let actions = orchestrator.build_actions(context, workflow_graph.clone());
        let actions = actions.collect::<Vec<_>>().await;

        assert_eq!(actions.len(), 2);
        let action0 = actions[0].as_ref().unwrap();
        assert_eq!(
            action0.loc,
            Location::from_node_index_iter([subworkflow_input_idx])
        );
        assert_eq!(
            action0.kind,
            ActionKind::LoadInput {
                name: "subworkflow".to_string()
            }
        );
        let action1 = actions[1].as_ref().unwrap();
        assert_eq!(action1.loc, Location::from_node_index_iter([a_input_idx]));
        assert_eq!(
            action1.kind,
            ActionKind::LoadInput {
                name: "a".to_string()
            }
        );

        orchestrator.perform_actions(stream::iter(actions)).await?;
        let subworkflow_input_complete_event = stream.next().await.unwrap();
        let subworkflow_input_complete_outputs =
            subworkflow_input_complete_event.clone().outputs().unwrap();
        assert_registry_contains_values(
            &registry,
            &orchestrator.default_storage_name,
            &subworkflow_input_complete_outputs,
            json!({"subworkflow": subworkflow_graph}),
        );

        let a_input_complete_event = stream.next().await.unwrap();
        let a_input_complete_outputs = a_input_complete_event.clone().outputs().unwrap();
        assert_registry_contains_values(
            &registry,
            &orchestrator.default_storage_name,
            &a_input_complete_outputs,
            json!({"a": 1}),
        );

        workflow_state
            .write(subworkflow_input_complete_event)
            .await?;
        workflow_state.write(a_input_complete_event).await?;
        let context = ExecutionContext {
            subworkflow_context: Location::root(),
            graph_inputs: inputs.clone(),
            workflow_state: Arc::clone(&workflow_state),
        };
        let actions = orchestrator.build_actions(context, workflow_graph.clone());
        let actions = actions.collect::<Vec<_>>().await;

        assert_eq!(actions.len(), 1);
        let action0 = actions[0].as_ref().unwrap();
        assert_eq!(
            action0.loc,
            Location::from_node_index_iter([eval_idx, inner_a_input_idx])
        );
        assert_eq!(
            action0.kind,
            ActionKind::LoadInput {
                name: "a".to_string()
            }
        );

        orchestrator.perform_actions(stream::iter(actions)).await?;
        let inner_a_input_complete_event = stream.next().await.unwrap();
        let inner_a_input_complete_outputs =
            inner_a_input_complete_event.clone().outputs().unwrap();
        assert_registry_contains_values(
            &registry,
            &orchestrator.default_storage_name,
            &inner_a_input_complete_outputs,
            json!({"a": 1}),
        );

        workflow_state.write(inner_a_input_complete_event).await?;
        let context = ExecutionContext {
            subworkflow_context: Location::root(),
            graph_inputs: inputs.clone(),
            workflow_state: Arc::clone(&workflow_state),
        };
        let actions = orchestrator.build_actions(context, workflow_graph.clone());
        let actions = actions.collect::<Vec<_>>().await;

        orchestrator.perform_actions(stream::iter(actions)).await?;
        let inner_output_complete_event = stream.next().await.unwrap();
        let inner_output_complete_outputs = inner_output_complete_event.clone().outputs().unwrap();
        assert_registry_contains_values(
            &registry,
            &orchestrator.default_storage_name,
            &inner_output_complete_outputs,
            json!({"out": 1}),
        );

        // TODO: Does this represent what the updater will actually do?
        let mut eval_complete_event = inner_output_complete_event.clone();
        eval_complete_event.loc = Location::from_node_index_iter([eval_idx]);
        workflow_state.write(inner_output_complete_event).await?;
        workflow_state.write(eval_complete_event).await?;
        let context = ExecutionContext {
            subworkflow_context: Location::root(),
            graph_inputs: inputs.clone(),
            workflow_state: Arc::clone(&workflow_state),
        };
        let actions = orchestrator.build_actions(context, workflow_graph.clone());
        let actions = actions.collect::<Vec<_>>().await;

        orchestrator.perform_actions(stream::iter(actions)).await?;
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
    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    async fn run_simple_task_workflow(#[case] default_storage_name: &str) -> miette::Result<()> {
        let mut workflow_graph = WorkflowGraph::new(["out".to_string()]);
        let input_idx = workflow_graph.add_node(
            NodeDefinition::Input {
                name: "a".to_string(),
            },
            [],
            ["a".to_string()],
        );
        let const_idx = workflow_graph.add_node(
            NodeDefinition::Const { value: 3.into() },
            [],
            ["value".to_string()],
        );
        let add_idx = workflow_graph.add_node(
            NodeDefinition::Task {
                worker_name: "builtin".to_string(),
                task_name: "iadd".to_string(),
            },
            ["a".to_string(), "b".to_string()],
            ["value".to_string()],
        );

        let output_idx = workflow_graph.output_idx();
        workflow_graph
            .link_nodes_by_port_name(&input_idx, "a", &add_idx, "a")
            .unwrap();
        workflow_graph
            .link_nodes_by_port_name(&const_idx, "value", &add_idx, "b")
            .unwrap();
        workflow_graph
            .link_nodes_by_port_name(&add_idx, "value", &output_idx, "out")
            .unwrap();

        let (registry, input_sets, _dir) = test_storage_registry(vec![json!({"a": 1})], vec![]);
        let executor_registry = test_executor_registry(&registry);
        let orchestrator = Orchestrator::try_new(
            &registry,
            &executor_registry,
            default_storage_name,
            "memory",
        )?;

        let (workflow_state, state_events) = InMemoryWorkflowState::test();
        let workflow_state: Arc<dyn WorkflowState> = Arc::new(workflow_state);
        let inputs = input_sets[0].clone();
        let context = ExecutionContext {
            subworkflow_context: Location::root(),
            graph_inputs: inputs.clone(),
            workflow_state: Arc::clone(&workflow_state),
        };
        let updater = Updater::new(Arc::clone(&workflow_state));
        let stream = orchestrator.listen()?;
        let _task = tokio::spawn(async move {
            updater.process(stream).await.unwrap();
            println!("done listening");
        });
        let actions = orchestrator.build_actions(context.clone(), workflow_graph.clone());
        orchestrator.perform_actions(actions).await?;

        let mut state_chunks = state_events.ready_chunks(8);
        while let Some(chunk) = state_chunks.next().await {
            if chunk.iter().any(|updated| updated.stopped) {
                break;
            }
            let actions = orchestrator.build_actions(context.clone(), workflow_graph.clone());
            orchestrator.perform_actions(actions).await?;
        }

        let a_input_state = workflow_state
            .read(&Location::from_node_index_iter([input_idx]))
            .await?;

        assert!(matches!(
            a_input_state,
            NodeState {
                complete_time: Some(..),
                outputs: Some(..),
                ..
            }
        ));

        let subworkflow_input_state = workflow_state
            .read(&Location::from_node_index_iter([const_idx]))
            .await?;

        assert!(matches!(
            subworkflow_input_state,
            NodeState {
                complete_time: Some(..),
                outputs: Some(..),
                ..
            }
        ));

        let output_state = workflow_state
            .read(&Location::from_node_index_iter([output_idx]))
            .await?;

        let outputs = output_state.outputs.expect("no outputs found");

        assert_registry_contains_values(
            &registry,
            default_storage_name,
            &outputs,
            json!({"out": 4}),
        );

        Ok(())
    }

    // Test that we can plan the first actions of a workflow and run them.
    #[rstest]
    #[case("memory")]
    #[case("file")]
    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    async fn run_simple_eval_workflow(#[case] default_storage_name: &str) -> miette::Result<()> {
        let mut subworkflow_graph = WorkflowGraph::new(["out".to_string()]);
        let inner_a_input_idx = subworkflow_graph.add_node(
            NodeDefinition::Input {
                name: "a".to_string(),
            },
            [],
            ["a".to_string()],
        );

        subworkflow_graph
            .link_nodes_by_port_name(
                &inner_a_input_idx,
                "a",
                &subworkflow_graph.output_idx(),
                "out",
            )
            .unwrap();

        let mut workflow_graph = WorkflowGraph::new(["out".to_string()]);
        let a_input_idx = workflow_graph.add_node(
            NodeDefinition::Input {
                name: "a".to_string(),
            },
            [],
            ["a".to_string()],
        );
        let subworkflow_input_idx = workflow_graph.add_node(
            NodeDefinition::Input {
                name: "subworkflow".to_string(),
            },
            [],
            ["subworkflow".to_string()],
        );
        let eval_idx = workflow_graph.add_node(
            NodeDefinition::Eval {},
            ["graph".to_string(), "a".to_string()],
            ["out".to_string()],
        );

        workflow_graph
            .link_nodes_by_port_name(&a_input_idx, "a", &eval_idx, "a")
            .unwrap();
        workflow_graph
            .link_nodes_by_port_name(&subworkflow_input_idx, "subworkflow", &eval_idx, "graph")
            .unwrap();
        workflow_graph
            .link_nodes_by_port_name(&eval_idx, "out", &workflow_graph.output_idx(), "out")
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

        let (workflow_state, state_events) = InMemoryWorkflowState::test();
        let workflow_state: Arc<dyn WorkflowState> = Arc::new(workflow_state);
        let inputs = input_sets[0].clone();
        let context = ExecutionContext {
            subworkflow_context: Location::root(),
            graph_inputs: inputs.clone(),
            workflow_state: Arc::clone(&workflow_state),
        };
        let updater = Updater::new(Arc::clone(&workflow_state));
        let stream = orchestrator.listen()?;
        let _task = tokio::spawn(async move {
            updater.process(stream).await.unwrap();
            println!("done listening");
        });
        let actions = orchestrator.build_actions(context.clone(), workflow_graph.clone());
        orchestrator.perform_actions(actions).await?;
        let mut state_chunks = state_events.ready_chunks(8);
        while let Some(chunk) = state_chunks.next().await {
            if chunk.iter().any(|updated| updated.stopped) {
                break;
            }
            let actions = orchestrator.build_actions(context.clone(), workflow_graph.clone());
            orchestrator.perform_actions(actions).await?;
        }

        let a_input_state = workflow_state
            .read(&Location::from_node_index_iter([a_input_idx]))
            .await?;

        assert!(matches!(
            a_input_state,
            NodeState {
                complete_time: Some(..),
                outputs: Some(..),
                ..
            }
        ));

        let subworkflow_input_state = workflow_state
            .read(&Location::from_node_index_iter([subworkflow_input_idx]))
            .await?;

        assert!(matches!(
            subworkflow_input_state,
            NodeState {
                complete_time: Some(..),
                outputs: Some(..),
                ..
            }
        ));

        let output_state = workflow_state
            .read(&Location::from_node_index_iter([
                workflow_graph.output_idx()
            ]))
            .await?;

        let outputs = output_state.outputs.expect("no outputs found");

        assert_registry_contains_values(
            &registry,
            &orchestrator.default_storage_name,
            &outputs,
            json!({"out": 1}),
        );

        Ok(())
    }

    // Test that we can plan the first actions of a workflow and run them.
    #[rstest]
    #[case("memory")]
    #[case("file")]
    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    async fn run_serialized_workflow(#[case] default_storage_name: &str) -> miette::Result<()> {
        let serialized_graph = include_str!("../tests/cli/data/sample_graph");
        let graph: LegacyWorkflowGraph =
            serde_json::from_str(serialized_graph).into_diagnostic()?;
        let workflow_graph = graph.to_workflow_graph().unwrap();

        let (registry, _input_sets, _dir) = test_storage_registry(vec![], vec![]);
        let executor_registry = test_executor_registry(&registry);
        let orchestrator = Orchestrator::try_new(
            &registry,
            &executor_registry,
            default_storage_name,
            "memory",
        )?;

        let (workflow_state, state_events) = InMemoryWorkflowState::test();
        let workflow_state: Arc<dyn WorkflowState> = Arc::new(workflow_state);
        let context = ExecutionContext {
            subworkflow_context: Location::root(),
            graph_inputs: HashMap::new(),
            workflow_state: Arc::clone(&workflow_state),
        };
        let updater = Updater::new(Arc::clone(&workflow_state));
        let stream = orchestrator.listen()?;
        let _task = tokio::spawn(async move {
            updater.process(stream).await.unwrap();
            println!("done listening");
        });
        let actions = orchestrator.build_actions(context.clone(), workflow_graph.clone());
        orchestrator.perform_actions(actions).await?;
        let mut state_chunks = state_events.ready_chunks(8);
        while let Some(chunk) = state_chunks.next().await {
            if chunk.iter().any(|updated| updated.stopped) {
                break;
            }
            let actions = orchestrator.build_actions(context.clone(), workflow_graph.clone());
            orchestrator.perform_actions(actions).await?;
        }

        let output_state = workflow_state
            .read(&Location::from_node_index_iter([
                workflow_graph.output_idx()
            ]))
            .await?;

        let outputs = output_state.outputs.expect("no outputs found");

        assert_registry_contains_values(
            &registry,
            &orchestrator.default_storage_name,
            &outputs,
            json!({"simple_eval_output": 12}),
        );

        Ok(())
    }
}
