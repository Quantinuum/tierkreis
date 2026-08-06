/*!
This module defines the [`NexusExecutor`] struct which implements [Executor]
by running tasks via the Nexus HTTP API.
*/
mod client;

use std::{
    collections::{HashMap, HashSet},
    io::Cursor,
    sync::{Arc, Mutex},
};

use futures::{
    FutureExt, SinkExt, StreamExt, TryStreamExt,
    channel::mpsc,
    future::{self, BoxFuture},
    stream::{BoxStream, FuturesUnordered},
};
use hugr::{envelope::read_envelope, extension::ExtensionRegistry};
use miette::{IntoDiagnostic, miette};
use tokio::task::{AbortHandle, JoinHandle};
use uuid::Uuid;

use crate::{
    asset_storage::{
        AssetSpec, AssetStorageRegistry, load_assets, reserve_asset_specs, save_asset_with_spec,
    },
    event::{
        EventReceiver, EventSender, RuntimeEvent, send_cancelled, send_complete, send_running,
    },
    executor::{
        Executor,
        interface::{TaskPlan, WorkerSpec},
        nexus::client::{
            NexusClient,
            models::jobs::{JobDefinition, StatusEnum},
        },
    },
    location::Location,
};
pub use client::NexusClientConfig;

struct BackgroundTaskPlan {
    workflow_run_id: Uuid,
    attempt: u32,
    loc: Location,
    job_id: Uuid,
    outputs: HashMap<String, AssetSpec>,
}

struct BackgroundTask {
    workflow_run_id: Uuid,
    attempt: u32,
    loc: Location,
    job_id: Uuid,
    outputs: HashMap<String, AssetSpec>,
}

type TaskSender = mpsc::Sender<BackgroundTaskPlan>;
type TaskReceiver = mpsc::Receiver<BackgroundTaskPlan>;
type CancelSender = mpsc::Sender<(Uuid, u32, Location)>;
type CancelReceiver = mpsc::Receiver<(Uuid, u32, Location)>;

type RunningFutures = FuturesUnordered<JoinHandle<miette::Result<BackgroundTask>>>;
type AbortHandles = HashMap<(Uuid, u32, Location), AbortHandle>;

async fn process_cancelled_task(
    _client: &NexusClient,
    event_sender: &mut EventSender,
    abort_handles: &mut AbortHandles,
    workflow_run_id: Uuid,
    attempt: u32,
    loc: Location,
) -> miette::Result<()> {
    // TODO: Call the job cancel endpoint
    // client.post(format!(
    //     "wss://nexus.quantinuum.com/api/jobs/v1beta3/{}/rpc/cancel",
    //     internal_task.job_id
    // ));
    let handle = abort_handles.remove(&(workflow_run_id, attempt, loc.clone()));
    if let Some(handle) = handle {
        handle.abort();
        send_cancelled(event_sender, workflow_run_id, attempt, loc).await?;
    }
    Ok(())
}

async fn process_finished_task(
    client: &NexusClient,
    event_sender: &mut EventSender,
    abort_handles: &mut AbortHandles,
    asset_storage_registry: &AssetStorageRegistry,
    background_task: BackgroundTask,
) -> miette::Result<()> {
    let loc = background_task.loc;
    let outputs = background_task.outputs;
    let workflow_run_id = background_task.workflow_run_id;
    let attempt = background_task.attempt;

    abort_handles.remove(&(workflow_run_id, attempt, loc.clone()));

    // Tokens may have expired at this point.
    client.refresh_tokens().await?;
    let job = client.get_job(background_task.job_id).await?;

    match job.status_enum() {
        StatusEnum::Completed => {}
        _ => todo!(),
    }

    let JobDefinition::ExecuteJobDefinition { items } = job.definition();

    for item in items {
        let result = client.get_qsys_result_chunk(item.result_id(), 0).await?;
        let result = serde_json::to_vec(result.results_ref()).into_diagnostic()?;
        save_asset_with_spec(
            asset_storage_registry,
            outputs.get("results").unwrap(), // TODO: NO UNWRAP!
            result,
        )
        .await?;
    }

    send_complete(
        event_sender,
        workflow_run_id,
        attempt,
        vec![loc],
        vec![outputs],
    )
    .await?;

    Ok(())
}

fn monitor_task(
    client: &NexusClient,
    event_sender: &EventSender,
    abort_handles: &mut AbortHandles,
    running: &mut RunningFutures,
    internal_task: BackgroundTaskPlan,
) {
    let loc = internal_task.loc;
    let workflow_run_id = internal_task.workflow_run_id;
    let attempt = internal_task.attempt;
    let job_id = internal_task.job_id;

    let background_loc = loc.clone();
    let outputs = internal_task.outputs;
    let event_sender = event_sender.clone();

    let client = client.clone();
    let mut event_sender = event_sender.clone();
    let task = tokio::task::spawn(async move {
        let mut job_status_stream = client.listen_for_job_status(internal_task.job_id).await?;

        // TODO: Handle disconnects here?
        while let Some(job_status) = job_status_stream.try_next().await? {
            match job_status.status() {
                StatusEnum::Submitted => {}
                StatusEnum::Queued | StatusEnum::Running => {
                    send_running(
                        &mut event_sender,
                        workflow_run_id,
                        attempt,
                        background_loc.clone(),
                    )
                    .await?;
                }
                StatusEnum::Completed | StatusEnum::Error => break,
                _ => todo!(),
            }
        }
        job_status_stream.close().await?;

        Ok(BackgroundTask {
            workflow_run_id,
            attempt,
            loc: background_loc,
            job_id,
            outputs,
        })
    });

    abort_handles.insert((workflow_run_id, attempt, loc), task.abort_handle());
    running.push(task);
}

async fn process_tasks(
    client: NexusClient,
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
                process_cancelled_task(&client, &mut event_sender, &mut abort_handles, workflow_run_id, attempt, loc)
                    .await
                    .expect("Failed to cancel task");
            }
            // A task has completed
            Some(res) = running.next() => {
                let background_task = match res {
                    Ok(ok) => ok,
                    Err(err) => panic!("Failed to join to future: {err}"),
                };

                // TODO: Send error event here
                let background_task = background_task.unwrap();
                // let background_task = match background_task {
                //     Ok(ok) => ok,
                //     Err(err) => send_error(&mut event_sender).await.expect("Failed to send error event"),
                // };

                process_finished_task(
                    &client,
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
                monitor_task(
                    &client,
                    &event_sender,
                    &mut abort_handles,
                    &mut running,
                    internal_task,
                );
            }
            else => break
        }
    }
}

/// [`SubprocessExecutor`] defines an [Executor] that performs Task Nodes using the Nexus HTTP API.
pub struct NexusExecutor {
    client: NexusClient,
    task_sender: TaskSender,
    cancel_sender: CancelSender,
    event_receiver: Mutex<Option<EventReceiver>>,
    output_storage_name: String,
    asset_storage_registry: AssetStorageRegistry,
}

impl NexusExecutor {
    /// Try to create a new [`NexusExecutor`] with an [`AssetStorageRegistry`], a
    /// configured name for an [`AssetStorage`][crate::asset_storage::AssetStorage]
    /// in the registry that determines where Assets are saved by default.
    ///
    /// # Errors
    ///
    /// This function will return Err if the specified `output_storage_name` does not
    /// exist inside the [`AssetStorageRegistry`] or if the [`NexusClient`] cannot
    /// be initialized.
    pub async fn try_new(
        client_config: &NexusClientConfig,
        asset_storage_registry: &AssetStorageRegistry,
        output_storage_name: &str,
    ) -> miette::Result<Self> {
        let client = NexusClient::try_new(client_config).await?;

        let asset_storage_registry_lock = asset_storage_registry.read().await;
        if !asset_storage_registry_lock.contains_key(output_storage_name) {
            return Err(miette!("output_storage_name not in registry"));
        }

        let background_asset_storage_registry = Arc::clone(asset_storage_registry);
        let (task_sender, task_receiver) = mpsc::channel(64);
        let (event_sender, event_receiver) = mpsc::channel(64);
        let (cancel_sender, cancel_receiver) = mpsc::channel(64);
        tokio::spawn(process_tasks(
            client.clone(),
            task_receiver,
            cancel_receiver,
            event_sender,
            background_asset_storage_registry,
        ));

        let asset_storage_registry = Arc::clone(asset_storage_registry);
        Ok(Self {
            client,
            task_sender,
            cancel_sender,
            event_receiver: Mutex::new(Some(event_receiver)),
            output_storage_name: output_storage_name.to_string(),
            asset_storage_registry,
        })
    }

    async fn build_outputs(
        &self,
        output_storage_name: &str,
        outputs: HashSet<String>,
    ) -> miette::Result<HashMap<String, AssetSpec>> {
        let output_specs = reserve_asset_specs(
            &self.asset_storage_registry,
            output_storage_name,
            outputs.len(),
        )
        .await?;
        let outputs: HashMap<String, AssetSpec> = outputs.into_iter().zip(output_specs).collect();
        Ok(outputs)
    }

    async fn upload_hugr(&self, project_id: Uuid, hugr_package: &[u8]) -> miette::Result<Uuid> {
        let (_, package) = read_envelope(Cursor::new(hugr_package), &ExtensionRegistry::new([]))
            .into_diagnostic()?;

        let hugr_data = self
            .client
            .new_hugr_data(
                "tkr-example-hugr",
                Some("A HUGR package uploaded with Tierkreis"),
                project_id,
                package,
            )
            .await?;

        Ok(hugr_data.id())
    }

    async fn start_single_job(
        &self,
        project_id: Uuid,
        job_name: &str,
        program_id: Uuid,
        n_shots: u64,
    ) -> miette::Result<Uuid> {
        let job_data = self
            .client
            .new_job_data(
                job_name,
                Some("trying new executor"),
                project_id,
                [(program_id, n_shots)],
            )
            .await?;

        Ok(job_data.id())
    }
}

fn extract_json_input<T: for<'b> serde::Deserialize<'b>>(
    inputs: &mut HashMap<String, Vec<u8>>,
    name: &str,
) -> miette::Result<T> {
    let input_bytes = inputs
        .remove(name)
        .ok_or_else(|| miette!("Missing input: {name}"))?;
    let input: T = serde_json::from_slice(&input_bytes).into_diagnostic()?;
    Ok(input)
}

impl Executor for NexusExecutor {
    fn workers(&self) -> BoxFuture<'_, miette::Result<Vec<WorkerSpec>>> {
        future::ok(vec![WorkerSpec {
            worker_name: "nexus_worker".to_string(),
        }])
        .boxed()
    }

    fn execute(&self, task_plans: Vec<TaskPlan>) -> BoxFuture<'_, miette::Result<()>> {
        async move {
            let mut task_sender = self.task_sender.clone();
            self.client.refresh_tokens().await?;

            for task_plan in task_plans {
                if task_plan.worker_name != "nexus_worker" {
                    return Err(miette!("Unknown worker: `{}`", task_plan.worker_name));
                }

                if task_plan.task_name != "submit_and_run" {
                    return Err(miette!("Unknown task: `{}`", task_plan.task_name));
                }

                let output_storage_name = task_plan
                    .output_storage_name
                    .clone()
                    .unwrap_or_else(|| self.output_storage_name.clone());

                let outputs = self
                    .build_outputs(&output_storage_name, task_plan.outputs)
                    .await?;

                let mut inputs =
                    load_assets(&self.asset_storage_registry, &task_plan.inputs).await?;

                let project_name: String = extract_json_input(&mut inputs, "project_name")?;
                let job_name: String = extract_json_input(&mut inputs, "job_name")?;
                let n_shots: u64 = extract_json_input(&mut inputs, "n_shots")?;

                let project_data = self
                    .client
                    .find_or_create_project_data(&project_name, Some("trying new executor"))
                    .await?;

                let hugr_package = inputs
                    .get("hugr_package")
                    .ok_or_else(|| miette!("Missing input: hugr_package"))?;
                let hugr_id = self.upload_hugr(project_data.id(), hugr_package).await?;

                let job_id = self
                    .start_single_job(project_data.id(), &job_name, hugr_id, n_shots)
                    .await?;

                task_sender
                    .send(BackgroundTaskPlan {
                        workflow_run_id: task_plan.workflow_run_id,
                        attempt: task_plan.attempt,
                        loc: task_plan.loc,
                        job_id,
                        outputs,
                    })
                    .await
                    .into_diagnostic()?;
            }

            Ok(())
        }
        .boxed()
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
        task_locations: Vec<crate::location::Location>,
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
    use axum::{
        Json, Router,
        extract::{
            WebSocketUpgrade,
            ws::{Message, WebSocket},
        },
    };
    use hugr::{
        builder::{FunctionBuilder, HugrBuilder},
        envelope::{EnvelopeConfig, write_envelope},
        package::Package,
        types::Signature,
    };
    use serde_json::json;

    use crate::{
        asset_storage::{assert_registry_contains_values, save_asset, test_storage_registry},
        event::{NodeEvent, NodeStatus, WorkflowRunEvent},
        executor::nexus::client::TLSMode,
    };

    use super::*;

    fn test_app() -> Router {
        async fn send_message(socket: &mut WebSocket, message: &serde_json::Value) {
            let msg = Message::text(serde_json::to_string(message).expect("failed to serialize"));
            socket.send(msg).await.expect("failed to send");
        }

        async fn handle_test_socket(mut socket: WebSocket) {
            send_message(
                &mut socket,
                &json!({"status": "SUBMITTED", "message": "job is submitted"}),
            )
            .await;
            send_message(
                &mut socket,
                &json!({"status": "QUEUED", "message": "job is queued"}),
            )
            .await;
            send_message(
                &mut socket,
                &json!({"status": "RUNNING", "message": "job is running"}),
            )
            .await;
            send_message(
                &mut socket,
                &json!({"status": "COMPLETED", "message": "job is completed"}),
            )
            .await;
        }

        Router::new()
            .route("/auth/tokens/refresh", axum::routing::post(|| async {}))
            .route(
                "/api/projects/v1beta2",
                axum::routing::get(|| async {
                    let id = Uuid::now_v7();
                    Json(json!({"data": [{"id": id}]}))
                }),
            )
            .route(
                "/api/hugr/v1beta",
                axum::routing::post(|| async {
                    let id = Uuid::now_v7();
                    Json(json!({"data": {"id": id}}))
                }),
            )
            .route(
                "/api/jobs/v1beta3",
                axum::routing::post(|| async {
                    let id = Uuid::now_v7();
                    Json(json!({"data": {"id": id}}))
                }),
            )
            .route(
                "/api/jobs/v1beta3/{job_id}/attributes/status/ws",
                axum::routing::get(|ws: WebSocketUpgrade| async {
                    ws.on_upgrade(handle_test_socket)
                }),
            )
            .route(
                "/api/jobs/v1beta3/{job_id}",
                axum::routing::get(|| async {
                    let id = Uuid::now_v7();
                    Json(json!({
                        "data": {
                            "attributes": {
                                "status": {
                                    "status": "COMPLETED",
                                    "message": "job has completed",
                                },
                                "definition": {
                                    "job_definition_type": "execute_job_definition",
                                    "items": [{"result_id": id}],
                                },
                            },
                        },
                    }))
                }),
            )
            .route(
                "/api/qsys_results/v1beta2/partial/{result_id}",
                axum::routing::get(|| async {
                    Json(json!({
                        "data": {
                            "attributes": {
                                "results": [[], [], [], [], []],
                            },
                        },
                    }))
                }),
            )
    }

    #[tokio::test]
    async fn execute_hugr() -> miette::Result<()> {
        let app = test_app();

        let listener = tokio::net::TcpListener::bind("0.0.0.0:8000")
            .await
            .into_diagnostic()?;

        tokio::spawn(async { axum::serve(listener, app).await.unwrap() });

        let (registry, input_sets, _temp_dir) = test_storage_registry(
            vec![json!({
                "n_shots": 5,
                "project_name": "tkr-demo",
                "job_name": "tkr-example-job",
            })],
            vec![],
        )
        .await;

        let hugr = FunctionBuilder::new("main", Signature::new(vec![], vec![]))
            .into_diagnostic()?
            .finish_hugr()
            .into_diagnostic()?;

        let package = Package::new([hugr]);
        let mut buf = Vec::new();
        write_envelope(&mut buf, &package, EnvelopeConfig::binary()).into_diagnostic()?;
        let hugr_asset_spec = save_asset(&registry, "memory", buf).await?;

        let mut inputs = input_sets[0].clone();
        inputs.insert("hugr_package".to_string(), hugr_asset_spec);

        let mut outputs = HashSet::new();
        outputs.insert("results".to_string());

        let token_dir = client::tests::setup_temp_tokens().await?;
        let token_path = token_dir.path();
        let output_storage_name = "memory";
        let config = NexusClientConfig {
            tls_mode: TLSMode::None,
            host: "localhost:8000".to_string(),
            token_dir: Some(token_path.to_path_buf()),
        };
        let executor = NexusExecutor::try_new(&config, &registry, output_storage_name).await?;
        let stream = executor.listen()?;
        executor
            .execute(vec![TaskPlan {
                worker_name: "nexus_worker".to_string(),
                task_name: "submit_and_run".to_string(),
                inputs,
                outputs,
                ..Default::default()
            }])
            .await?;

        let events = stream.take(3).collect::<Vec<_>>().await;
        assert_eq!(events.len(), 3);
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
                    status: NodeStatus::Running { .. },
                    ..
                }),
                ..
            }
        ));
        assert!(matches!(
            events[2],
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
            output_storage_name,
            &events[2].clone().outputs()[0],
            json!({"results": [[], [], [], [], []]}),
        )
        .await;

        Ok(())
    }
}
