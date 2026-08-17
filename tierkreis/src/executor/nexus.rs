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
use miette::{Context, IntoDiagnostic, miette};
use tracing::{Instrument, instrument, warn};
use uuid::Uuid;

use crate::{
    asset_storage::{
        AssetSpec, AssetStorageRegistry, load_assets, reserve_asset_specs, save_asset_with_spec,
    },
    event::{
        EventReceiver, EventSender, RuntimeEvent, send_cancelled, send_complete, send_error,
        send_running,
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

#[derive(Clone)]
struct BackgroundTaskPlan {
    workflow_run_id: Uuid,
    attempt: u32,
    loc: Location,
    job_id: Uuid,
    outputs: HashMap<String, AssetSpec>,
    parent_span: tracing::Span,
}

#[derive(Clone)]
struct BackgroundTask {
    workflow_run_id: Uuid,
    attempt: u32,
    loc: Location,
    job_id: Uuid,
    outputs: HashMap<String, AssetSpec>,
}

enum InternalJobMonitoringStatus {
    Watching { job_id: Uuid },
    Cancelling,
}

type TaskSender = mpsc::Sender<BackgroundTaskPlan>;
type TaskReceiver = mpsc::Receiver<BackgroundTaskPlan>;
type CancelSender = mpsc::Sender<(Uuid, u32, Location)>;
type CancelReceiver = mpsc::Receiver<(Uuid, u32, Location)>;

type RunningFutures<'a> = FuturesUnordered<BoxFuture<'a, (BackgroundTask, miette::Result<()>)>>;
// Mapping of Node to the internal monitoring status of the Node..
type JobHandles = HashMap<(Uuid, u32, Location), InternalJobMonitoringStatus>;

#[instrument(skip_all, err)]
async fn process_cancelled_task(
    client: &NexusClient,
    job_handles: &mut JobHandles,
    workflow_run_id: Uuid,
    attempt: u32,
    loc: Location,
) -> miette::Result<()> {
    let key = (workflow_run_id, attempt, loc.clone());
    let handle = job_handles.remove(&key);
    match handle {
        // We know the job id, cancel the job.
        Some(InternalJobMonitoringStatus::Watching { job_id }) => {
            tracing::info!("Cancelling job for node: {key:?}");
            client.cancel_job(job_id).await?;
        }
        // We are already planning to cancel the job if we see this node.
        Some(InternalJobMonitoringStatus::Cancelling) => {}
        // Register that the node needs to be cancelled if we see it.
        None => {
            warn!("Got cancel request before registering job_id for node: {key:?}");
            job_handles.insert(key, InternalJobMonitoringStatus::Cancelling);
        }
    }
    Ok(())
}

#[instrument(skip_all, err)]
async fn process_finished_task(
    client: &NexusClient,
    event_sender: &mut EventSender,
    job_handles: &mut JobHandles,
    asset_storage_registry: &AssetStorageRegistry,
    background_task: BackgroundTask,
) -> miette::Result<()> {
    let loc = background_task.loc;
    let outputs = background_task.outputs;
    let workflow_run_id = background_task.workflow_run_id;
    let attempt = background_task.attempt;

    job_handles.remove(&(workflow_run_id, attempt, loc.clone()));

    // Tokens may have expired at this point.
    client.refresh_tokens().await?;
    let job = client.get_job(background_task.job_id).await?;

    match job.status_enum() {
        StatusEnum::Completed => {
            let JobDefinition::ExecuteJobDefinition { items } = job.definition();

            for item in items {
                let result_id = item
                    .result_id()
                    .ok_or_else(|| miette!("Item is complete but has no `result_id`"))?;

                let mut full_result = Vec::new();
                let mut chunk_number = 0;
                while let Some(result) = client
                    .get_qsys_result_chunk(result_id, chunk_number)
                    .await?
                {
                    full_result.extend(result.results());
                    chunk_number += 1;
                }
                let result = serde_json::to_vec(&full_result).into_diagnostic()?;
                save_asset_with_spec(
                    asset_storage_registry,
                    outputs
                        .get("results")
                        .ok_or_else(|| miette!("Missing output: `results`"))?,
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
        }
        StatusEnum::Cancelled => {
            send_cancelled(event_sender, workflow_run_id, attempt, loc).await?;
        }
        StatusEnum::Error => {
            send_error(
                event_sender,
                workflow_run_id,
                attempt,
                loc,
                &miette!("Job failed with message: {}", job.status_message()),
            )
            .await?;
        }
        StatusEnum::Depleted => {
            send_error(
                event_sender,
                workflow_run_id,
                attempt,
                loc,
                &miette!("Job is depleted"),
            )
            .await?;
        }
        StatusEnum::Terminated => {
            send_error(
                event_sender,
                workflow_run_id,
                attempt,
                loc,
                &miette!("Job was terminated"),
            )
            .await?;
        }
        status => {
            send_error(
                event_sender,
                workflow_run_id,
                attempt,
                loc,
                &miette!("Unexpected status for finished job: {:?}", status),
            )
            .await?;
        }
    }

    Ok(())
}

#[instrument(skip_all, err)]
async fn monitor_task(
    client: &NexusClient,
    event_sender: &EventSender,
    cancel_sender: &mut CancelSender,
    job_handles: &mut JobHandles,
    running: &mut RunningFutures<'_>,
    internal_task: BackgroundTaskPlan,
) -> miette::Result<()> {
    let loc = internal_task.loc;
    let workflow_run_id = internal_task.workflow_run_id;
    let attempt = internal_task.attempt;
    let job_id = internal_task.job_id;

    let task_loc = loc.clone();
    let background_loc = loc.clone();
    let outputs = internal_task.outputs.clone();
    let event_sender = event_sender.clone();
    let parent_span = internal_task.parent_span.clone();

    let client = client.clone();
    let mut event_sender = event_sender.clone();
    let task = tokio::task::spawn(
        async move {
            let mut job_status_stream = client
                .listen_for_job_status(internal_task.job_id)
                .await
                .wrap_err("Failed to listen for job status")?;

            'outer: loop {
                while let Some(job_status) = job_status_stream
                    .try_next()
                    .await
                    .wrap_err("Failed while listening for job status")?
                {
                    match job_status.status() {
                        StatusEnum::Submitted | StatusEnum::Cancelling | StatusEnum::Retrying => {}
                        StatusEnum::Queued | StatusEnum::Running => {
                            send_running(
                                &mut event_sender,
                                workflow_run_id,
                                attempt,
                                task_loc.clone(),
                            )
                            .await?;
                        }
                        // Exit if the job status is a final state.
                        StatusEnum::Completed
                        | StatusEnum::Error
                        | StatusEnum::Terminated
                        | StatusEnum::Depleted
                        | StatusEnum::Cancelled => break 'outer,
                    }
                }

                // Reconnect if the job status was not a final state.
                tracing::info!("Reconnecting to job status stream");
                job_status_stream = client
                    .listen_for_job_status(internal_task.job_id)
                    .await
                    .wrap_err("Failed to listen for job status")?;
            }
            tracing::info!("Job status stream ended, processing finished task");
            job_status_stream
                .close()
                .await
                .wrap_err("Failed to close job status stream")?;

            Ok(())
        }
        .instrument(parent_span),
    )
    .map(move |result| {
        (
            BackgroundTask {
                workflow_run_id,
                attempt,
                loc: background_loc.clone(),
                job_id,
                outputs,
            },
            result
                .map_err(|err| miette!("Failed to join task: {err}"))
                .flatten(),
        )
    });

    let key = (workflow_run_id, attempt, loc.clone());
    // If we are meant to cancel this job, signal this to the cancellation routine.
    if let Some(InternalJobMonitoringStatus::Cancelling) = job_handles.get(&key) {
        cancel_sender.send(key.clone()).await.into_diagnostic()?;
    }
    job_handles.insert(key, InternalJobMonitoringStatus::Watching { job_id });
    running.push(task.boxed());
    Ok(())
}

async fn process_tasks(
    client: NexusClient,
    mut task_receiver: TaskReceiver,
    mut cancel_sender: CancelSender,
    mut cancel_receiver: CancelReceiver,
    mut event_sender: EventSender,
    asset_storage_registry: AssetStorageRegistry,
) {
    let mut job_handles: JobHandles = HashMap::new();
    let mut running: RunningFutures = FuturesUnordered::new();

    loop {
        tokio::select! {
            // A task has been cancelled
            Some((workflow_run_id, attempt, loc)) = cancel_receiver.next() => {
                tracing::debug!(workflow_run_id = %workflow_run_id, attempt = %attempt, loc = %loc, "Received cancel request");
                process_cancelled_task(&client, &mut job_handles, workflow_run_id, attempt, loc)
                    .await
                    .expect("Failed to cancel task");
            }
            // A task has completed
            Some((task, result)) = running.next() => {
                tracing::debug!(workflow_run_id = %task.workflow_run_id, attempt = %task.attempt, loc = %task.loc, "Task completed");
                if let Err(err) = result {
                    send_error(
                        &mut event_sender,
                        task.workflow_run_id,
                        task.attempt,
                        task.loc.clone(),
                        &err,
                    )
                    .await
                    .expect("Failed to send error event");
                }


                let processing_result = process_finished_task(
                    &client,
                    &mut event_sender,
                    &mut job_handles,
                    &asset_storage_registry,
                    task.clone(),
                )
                .await;
                if let Err(err) = processing_result {
                    send_error(
                        &mut event_sender,
                        task.workflow_run_id,
                        task.attempt,
                        task.loc,
                        &err,
                    )
                    .await
                    .expect("Failed to send error event");
                }
            }
            // A task has been submitted
            Some(internal_task) = task_receiver.next() => {
                tracing::debug!(workflow_run_id = %internal_task.workflow_run_id, attempt = %internal_task.attempt, loc = %internal_task.loc, "Received task to monitor");
                let res = monitor_task(
                    &client,
                    &event_sender,
                    &mut cancel_sender,
                    &mut job_handles,
                    &mut running,
                    internal_task.clone(),
                )
                .await;

                if let Err(err) = res {
                    send_error(
                        &mut event_sender,
                        internal_task.workflow_run_id,
                        internal_task.attempt,
                        internal_task.loc,
                        &err,
                    )
                    .await
                    .expect("Failed to send error event");
                }
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
            cancel_sender.clone(),
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

    async fn upload_hugr(
        &self,
        project_id: Uuid,
        hugr_name: &str,
        hugr_package: &[u8],
    ) -> miette::Result<Uuid> {
        // TODO: Requiring Extensions for the enveleope read here might be problematic.
        let (_, package) = read_envelope(Cursor::new(hugr_package), &ExtensionRegistry::new([]))
            .into_diagnostic()?;

        let hugr_data = self
            .client
            .new_hugr_data(hugr_name, None, project_id, package)
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
            .new_job_data(job_name, None, project_id, [(program_id, n_shots)])
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
                let hugr_id = self
                    .upload_hugr(project_data.id(), &job_name, hugr_package)
                    .await?;

                let job_id = self
                    .start_single_job(project_data.id(), &job_name, hugr_id, n_shots)
                    .await?;

                let parent_span = tracing::Span::current();
                task_sender
                    .send(BackgroundTaskPlan {
                        workflow_run_id: task_plan.workflow_run_id,
                        attempt: task_plan.attempt,
                        loc: task_plan.loc,
                        job_id,
                        outputs,
                        parent_span,
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
            Query, State, WebSocketUpgrade,
            ws::{Message, WebSocket},
        },
    };
    use axum_test::TestServer;
    use hugr::{
        builder::{FunctionBuilder, HugrBuilder},
        envelope::{EnvelopeConfig, write_envelope},
        package::Package,
        types::{Signature, Type},
    };
    use serde::Deserialize;
    use serde_json::json;
    use tempfile::TempDir;
    use tokio::sync::watch;

    use crate::{
        asset_storage::{assert_registry_contains_values, save_asset, test_storage_registry},
        event::{NodeEvent, NodeStatus, WorkflowRunEvent},
        executor::nexus::client::TLSMode,
    };

    use super::*;

    async fn test_config_for_app(
        app: Router,
    ) -> miette::Result<(NexusClientConfig, TestServer, TempDir)> {
        let server = axum_test::TestServer::builder().http_transport().build(app);
        let url = server.server_address().expect("No server address");

        let token_dir = client::tests::setup_temp_tokens().await?;
        let token_path = token_dir.path();
        let config = NexusClientConfig {
            tls_mode: TLSMode::None,
            host: format!(
                "{}:{}",
                url.host_str().expect("No host"),
                url.port().expect("No port")
            ),
            token_dir: Some(token_path.to_path_buf()),
        };

        Ok((config, server, token_dir))
    }

    fn empty_hugr() -> miette::Result<hugr::Hugr> {
        let hugr = FunctionBuilder::new("main", Signature::new(vec![], vec![]))
            .into_diagnostic()?
            .finish_hugr()
            .into_diagnostic()?;
        Ok(hugr)
    }

    fn invalid_hugr() -> miette::Result<hugr::Hugr> {
        let mut hugr_builder =
            FunctionBuilder::new("main", Signature::new(vec![], vec![])).into_diagnostic()?;
        hugr_builder.add_input(Type::UNIT).into_diagnostic()?;
        let hugr = hugr_builder.finish_hugr().into_diagnostic()?;
        Ok(hugr)
    }

    async fn save_test_hugr(
        registry: &AssetStorageRegistry,
        hugr: hugr::Hugr,
    ) -> miette::Result<AssetSpec> {
        let package = Package::new([hugr]);
        let mut buf = Vec::new();
        write_envelope(&mut buf, &package, EnvelopeConfig::binary()).into_diagnostic()?;
        let hugr_asset_spec = save_asset(registry, "memory", buf).await?;
        Ok(hugr_asset_spec)
    }

    async fn send_ws_message(socket: &mut WebSocket, message: &serde_json::Value) {
        let msg = Message::text(serde_json::to_string(message).expect("failed to serialize"));
        socket.send(msg).await.expect("failed to send");
    }

    fn base_test_app<S>() -> Router<S>
    where
        S: Clone + Send + Sync + 'static,
    {
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
    }

    fn happy_test_app() -> Router {
        async fn handle_test_socket(mut socket: WebSocket) {
            send_ws_message(
                &mut socket,
                &json!({"status": "SUBMITTED", "message": "job is submitted"}),
            )
            .await;
            send_ws_message(
                &mut socket,
                &json!({"status": "QUEUED", "message": "job is queued"}),
            )
            .await;
            send_ws_message(
                &mut socket,
                &json!({"status": "RUNNING", "message": "job is running"}),
            )
            .await;
            send_ws_message(
                &mut socket,
                &json!({"status": "COMPLETED", "message": "job is completed"}),
            )
            .await;
            socket.send(Message::Close(None)).await.unwrap();
        }

        #[derive(Deserialize)]
        struct ChunkParameters {
            chunk_number: u64,
        }

        base_test_app()
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
                axum::routing::get(|parameters: Query<ChunkParameters>| async move {
                    if parameters.chunk_number == 3 {
                        return Err(axum::http::StatusCode::NOT_FOUND);
                    }
                    Ok(Json(json!({
                        "data": {
                            "attributes": {
                                "results": [[], [], [], [], []],
                            },
                        },
                    })))
                }),
            )
    }

    /// Test a scenario where we execute a hugr.
    #[tokio::test]
    async fn execute_hugr() -> miette::Result<()> {
        let app = happy_test_app();
        let (config, _server, _token_dir) = test_config_for_app(app).await?;

        let (registry, input_sets, _temp_dir) = test_storage_registry(
            vec![json!({
                "n_shots": 5,
                "project_name": "tkr-demo",
                "job_name": "tkr-example-job",
            })],
            vec![],
        )
        .await;

        let hugr_asset_spec = save_test_hugr(&registry, empty_hugr()?).await?;

        let mut inputs = input_sets[0].clone();
        inputs.insert("hugr_package".to_string(), hugr_asset_spec);

        let mut outputs = HashSet::new();
        outputs.insert("results".to_string());

        let output_storage_name = "memory";
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
            json!({"results": [
                [], [], [], [], [],
                [], [], [], [], [],
                [], [], [], [], [],
            ]}),
        )
        .await;

        Ok(())
    }

    fn error_test_app() -> Router {
        async fn handle_test_socket(mut socket: WebSocket) {
            send_ws_message(
                &mut socket,
                &json!({"status": "SUBMITTED", "message": "job is submitted"}),
            )
            .await;
            send_ws_message(
                &mut socket,
                &json!({"status": "ERROR", "message": "job has errored"}),
            )
            .await;
            socket.send(Message::Close(None)).await.unwrap();
        }

        base_test_app()
            .route(
                "/api/jobs/v1beta3/{job_id}/attributes/status/ws",
                axum::routing::get(|ws: WebSocketUpgrade| async {
                    ws.on_upgrade(handle_test_socket)
                }),
            )
            .route(
                "/api/jobs/v1beta3/{job_id}",
                axum::routing::get(|| async {
                    Json(json!({
                        "data": {
                            "attributes": {
                                "status": {
                                    "status": "ERROR",
                                    "message": "job has errored",
                                },
                                "definition": {
                                    "job_definition_type": "execute_job_definition",
                                    "items": [{}],
                                },
                            },
                        },
                    }))
                }),
            )
    }

    // Test that we can launch a task and listen for
    // errors when they occur
    #[tokio::test]
    async fn execute_hugr_error() -> miette::Result<()> {
        let app = error_test_app();
        let (config, _server, _token_dir) = test_config_for_app(app).await?;

        let (registry, input_sets, _temp_dir) = test_storage_registry(
            vec![json!({
                "n_shots": 5,
                "project_name": "tkr-demo",
                "job_name": "tkr-example-job",
            })],
            vec![],
        )
        .await;

        let hugr_asset_spec = save_test_hugr(&registry, invalid_hugr()?).await?;

        let mut inputs = input_sets[0].clone();
        inputs.insert("hugr_package".to_string(), hugr_asset_spec);

        let mut outputs = HashSet::new();
        outputs.insert("results".to_string());

        let output_storage_name = "memory";
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

        let events = stream.take(1).collect::<Vec<_>>().await;
        assert_eq!(events.len(), 1);
        assert!(matches!(
            events[0],
            RuntimeEvent::WorkflowRun {
                event: WorkflowRunEvent::NodeEvent(NodeEvent {
                    status: NodeStatus::Error { ref error, .. },
                    ..
                }),
                ..
            } if error == "Job failed with message: job has errored"
        ));

        Ok(())
    }

    fn cancel_test_app() -> Router {
        #[derive(Clone)]
        struct AppState {
            send: watch::Sender<bool>,
            recv: watch::Receiver<bool>,
        }

        async fn handle_test_socket(mut socket: WebSocket, mut recv: watch::Receiver<bool>) {
            send_ws_message(
                &mut socket,
                &json!({"status": "SUBMITTED", "message": "job is submitted"}),
            )
            .await;
            // Wait for the job to be cancelled by a call to /rpc/cancel
            recv.wait_for(|cancelled| *cancelled).await.unwrap();
            send_ws_message(
                &mut socket,
                &json!({"status": "CANCELLED", "message": "job was cancelled"}),
            )
            .await;
            socket.send(Message::Close(None)).await.unwrap();
        }

        let (send, recv) = watch::channel(false);
        base_test_app()
            .route(
                "/api/jobs/v1beta3/{job_id}/attributes/status/ws",
                axum::routing::get(
                    |State(state): State<AppState>, ws: WebSocketUpgrade| async move {
                        let recv = state.recv.clone();
                        ws.on_upgrade(|socket| handle_test_socket(socket, recv))
                    },
                ),
            )
            .route(
                "/api/jobs/v1beta3/{job_id}",
                axum::routing::get(|| async {
                    Json(json!({
                        "data": {
                            "attributes": {
                                "status": {
                                    "status": "CANCELLED",
                                    "message": "job has cancelled",
                                },
                                "definition": {
                                    "job_definition_type": "execute_job_definition",
                                    "items": [{}],
                                },
                            },
                        },
                    }))
                }),
            )
            .route(
                "/api/jobs/v1beta3/{job_id}/rpc/cancel",
                axum::routing::post(|State(state): State<AppState>| async move {
                    state.send.send(true).unwrap();
                }),
            )
            .with_state(AppState { send, recv })
    }

    /// Test a scenario where we start a task and then cancel it immediately.
    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn execute_and_cancel_hugr() -> miette::Result<()> {
        let app = cancel_test_app();
        let (config, _server, _token_dir) = test_config_for_app(app).await?;

        let (registry, input_sets, _temp_dir) = test_storage_registry(
            vec![json!({
                "n_shots": 5,
                "project_name": "tkr-demo",
                "job_name": "tkr-example-job",
            })],
            vec![],
        )
        .await;

        let hugr_asset_spec = save_test_hugr(&registry, empty_hugr()?).await?;

        let mut inputs = input_sets[0].clone();
        inputs.insert("hugr_package".to_string(), hugr_asset_spec);

        let mut outputs = HashSet::new();
        outputs.insert("results".to_string());

        let output_storage_name = "memory";
        let executor = NexusExecutor::try_new(&config, &registry, output_storage_name).await?;
        let mut stream = executor.listen()?;

        let loc = Location::from_usize_iter([0]);
        executor
            .execute(vec![TaskPlan {
                loc: loc.clone(),
                worker_name: "nexus_worker".to_string(),
                task_name: "submit_and_run".to_string(),
                inputs,
                outputs,
                ..Default::default()
            }])
            .await?;

        executor.cancel(Uuid::nil(), 0, vec![loc]).await?;

        let event = stream.next().await.unwrap();
        dbg!(&event);
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
}
