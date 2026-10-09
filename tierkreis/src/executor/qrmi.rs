/*!
This module defines the [`QrmiExecutor`] implementation of [`Executor`] for
running tasks on a fixed QRMI quantum resource.
*/

use std::{
    collections::{HashMap, HashSet},
    sync::{Arc, Mutex},
    time::Duration,
};

use futures::{
    FutureExt, SinkExt, StreamExt,
    channel::mpsc,
    future::BoxFuture,
    stream::{BoxStream, FuturesUnordered},
};
use miette::{Context, IntoDiagnostic, miette};
use qrmi::{
    QuantumResource,
    models::{Payload, TaskStatus},
};
use serde::{Deserialize, Serialize};
use tokio::{sync::Mutex as AsyncMutex, task::AbortHandle};
use uuid::Uuid;

use crate::{
    asset_storage::{
        AssetSpec, AssetStorageRegistry, load_asset, reserve_asset_specs, save_asset_with_spec,
    },
    event::{
        EventReceiver, EventSender, RuntimeEvent, send_cancelled, send_complete, send_error,
        send_queued, send_running,
    },
    executor::{
        Executor,
        interface::{TaskPlan, WorkerSpec},
    },
    location::Location,
};

const WORKER_NAME: &str = "qrmi_worker";
const TASK_NAME: &str = "run";
const PAYLOAD_INPUT: &str = "payload";
const RESULT_OUTPUT: &str = "result";

type Resource = Arc<AsyncMutex<Box<dyn QuantumResource>>>;
type TaskKey = (Uuid, u32, Location);
type TaskSender = mpsc::Sender<BackgroundTask>;
type TaskReceiver = mpsc::Receiver<BackgroundTask>;
type CancelSender = mpsc::Sender<TaskKey>;
type CancelReceiver = mpsc::Receiver<TaskKey>;
type RunningTasks = FuturesUnordered<BoxFuture<'static, (TaskKey, miette::Result<()>)>>;

/// The QRMI resource implementation used by a [`QrmiExecutor`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum QrmiResourceType {
    /// IBM Quantum System.
    #[serde(rename = "ibm-quantum-system")]
    IBMQuantumSystem,
    /// The deprecated IBM Qiskit Runtime Service.
    #[serde(rename = "qiskit-runtime-service")]
    QiskitRuntimeService,
    /// IBM Quantum Compute Service.
    #[serde(rename = "ibm-quantum-compute-service")]
    IBMQuantumComputeService,
    /// Pasqal Cloud.
    #[serde(rename = "pasqal-cloud")]
    PasqalCloud,
    /// Pasqal Local.
    #[serde(rename = "pasqal-local")]
    PasqalLocal,
    /// Alice & Bob Felis.
    #[serde(rename = "alice-bob-felis")]
    AliceBobFelis,
    /// IQM Server.
    #[serde(rename = "iqm-server")]
    IQMServer,
    /// OQTOPUS Cloud.
    #[serde(rename = "oqtopus")]
    Oqtopus,
}

/// Configuration for a fixed QRMI quantum resource.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct QrmiExecutorConfig {
    /// The resource identifier or backend name understood by QRMI.
    pub resource_id: String,
    /// The QRMI resource implementation to construct.
    pub resource_type: QrmiResourceType,
    /// QRMI settings without the resource-name prefix. When empty, QRMI reads
    /// its settings from the process environment.
    #[serde(default)]
    pub environment: HashMap<String, String>,
    /// Delay between task status requests.
    #[serde(default = "default_poll_interval_secs")]
    pub poll_interval_secs: u64,
}

const fn default_poll_interval_secs() -> u64 {
    1
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum QrmiPayload {
    QiskitPrimitive {
        input: String,
        program_id: String,
    },
    PasqalCloud {
        sequence: String,
        job_runs: i32,
    },
    AliceBobFelis {
        human_qir: String,
        input_params: String,
    },
    IqmServer {
        iqmjson: String,
        job_type: String,
        use_timeslot: Option<bool>,
        tag: Option<String>,
    },
    Oqtopus {
        job_spec: String,
    },
}

impl QrmiPayload {
    fn validate_for(&self, resource_type: QrmiResourceType) -> miette::Result<()> {
        let compatible = matches!(
            (resource_type, self),
            (
                QrmiResourceType::IBMQuantumSystem
                    | QrmiResourceType::QiskitRuntimeService
                    | QrmiResourceType::IBMQuantumComputeService,
                Self::QiskitPrimitive { .. }
            ) | (
                QrmiResourceType::PasqalCloud | QrmiResourceType::PasqalLocal,
                Self::PasqalCloud { .. }
            ) | (QrmiResourceType::AliceBobFelis, Self::AliceBobFelis { .. })
                | (QrmiResourceType::IQMServer, Self::IqmServer { .. })
                | (QrmiResourceType::Oqtopus, Self::Oqtopus { .. })
        );
        if compatible {
            Ok(())
        } else {
            Err(miette!(
                "QRMI payload is incompatible with resource type {resource_type:?}"
            ))
        }
    }
}

impl From<QrmiPayload> for Payload {
    fn from(value: QrmiPayload) -> Self {
        match value {
            QrmiPayload::QiskitPrimitive { input, program_id } => {
                Self::QiskitPrimitive { input, program_id }
            }
            QrmiPayload::PasqalCloud { sequence, job_runs } => {
                Self::PasqalCloud { sequence, job_runs }
            }
            QrmiPayload::AliceBobFelis {
                human_qir,
                input_params,
            } => Self::AliceBobFelis {
                human_qir,
                input_params,
            },
            QrmiPayload::IqmServer {
                iqmjson,
                job_type,
                use_timeslot,
                tag,
            } => Self::IQMServer {
                iqmjson,
                job_type,
                use_timeslot,
                tag,
            },
            QrmiPayload::Oqtopus { job_spec } => Self::Oqtopus { job_spec },
        }
    }
}

#[derive(Clone)]
struct BackgroundTask {
    workflow_run_id: Uuid,
    attempt: u32,
    loc: Location,
    task_id: String,
    output: AssetSpec,
}

impl BackgroundTask {
    fn key(&self) -> TaskKey {
        (self.workflow_run_id, self.attempt, self.loc.clone())
    }
}

enum TaskRegistration {
    Watching(String),
    CancelBeforeRegistration,
}

fn construct_resource(config: &QrmiExecutorConfig) -> miette::Result<Box<dyn QuantumResource>> {
    let resource_id = &config.resource_id;
    let environment = &config.environment;
    let resource: Box<dyn QuantumResource> = match config.resource_type {
        QrmiResourceType::IBMQuantumSystem => Box::new(
            if environment.is_empty() {
                qrmi::ibm::IBMQuantumSystem::new(resource_id)
            } else {
                qrmi::ibm::IBMQuantumSystem::from_config(resource_id, environment.clone())
            }
            .into_diagnostic()?,
        ),
        QrmiResourceType::QiskitRuntimeService => Box::new(
            if environment.is_empty() {
                qrmi::ibm::IBMQiskitRuntimeService::new(resource_id)
            } else {
                qrmi::ibm::IBMQiskitRuntimeService::from_config(resource_id, environment.clone())
            }
            .into_diagnostic()?,
        ),
        QrmiResourceType::IBMQuantumComputeService => Box::new(
            if environment.is_empty() {
                qrmi::ibm::IBMQuantumComputeService::new(resource_id)
            } else {
                qrmi::ibm::IBMQuantumComputeService::from_config(resource_id, environment.clone())
            }
            .into_diagnostic()?,
        ),
        QrmiResourceType::PasqalCloud => Box::new(
            if environment.is_empty() {
                qrmi::pasqal::PasqalCloud::new(resource_id)
            } else {
                qrmi::pasqal::PasqalCloud::from_config(resource_id, environment.clone())
            }
            .into_diagnostic()?,
        ),
        QrmiResourceType::PasqalLocal => Box::new(
            if environment.is_empty() {
                qrmi::pasqal::PasqalLocal::new(resource_id)
            } else {
                qrmi::pasqal::PasqalLocal::from_config(resource_id, environment.clone())
            }
            .into_diagnostic()?,
        ),
        QrmiResourceType::AliceBobFelis => Box::new(
            if environment.is_empty() {
                qrmi::alice_bob::AliceBobFelis::new(resource_id)
            } else {
                qrmi::alice_bob::AliceBobFelis::from_config(resource_id, environment.clone())
            }
            .into_diagnostic()?,
        ),
        QrmiResourceType::IQMServer => Box::new(
            if environment.is_empty() {
                qrmi::iqm::IQMServer::new(resource_id)
            } else {
                qrmi::iqm::IQMServer::from_config(resource_id, environment.clone())
            }
            .into_diagnostic()?,
        ),
        QrmiResourceType::Oqtopus => Box::new(
            if environment.is_empty() {
                qrmi::oqtopus::Oqtopus::new(resource_id)
            } else {
                qrmi::oqtopus::Oqtopus::from_config(resource_id, environment.clone())
            }
            .into_diagnostic()?,
        ),
    };
    Ok(resource)
}

async fn monitor_task(
    resource: Resource,
    mut event_sender: EventSender,
    asset_storage_registry: AssetStorageRegistry,
    poll_interval: Duration,
    task: BackgroundTask,
) -> (TaskKey, miette::Result<()>) {
    let key = task.key();
    let result = async {
        send_queued(
            &mut event_sender,
            task.workflow_run_id,
            task.attempt,
            task.loc.clone(),
            Some(task.task_id.clone()),
        )
        .await?;

        let mut sent_running = false;
        loop {
            let status = resource
                .lock()
                .await
                .task_status(&task.task_id)
                .await
                .into_diagnostic()
                .wrap_err("Failed to get QRMI task status")?;
            match status {
                TaskStatus::Queued => {}
                TaskStatus::Running => {
                    if !sent_running {
                        send_running(
                            &mut event_sender,
                            task.workflow_run_id,
                            task.attempt,
                            task.loc.clone(),
                        )
                        .await?;
                        sent_running = true;
                    }
                }
                TaskStatus::Completed => {
                    let result = resource
                        .lock()
                        .await
                        .task_result(&task.task_id)
                        .await
                        .into_diagnostic()
                        .wrap_err("Failed to get QRMI task result")?;
                    save_asset_with_spec(
                        &asset_storage_registry,
                        &task.output,
                        result.value.into_bytes(),
                    )
                    .await?;
                    send_complete(
                        &mut event_sender,
                        task.workflow_run_id,
                        task.attempt,
                        vec![(
                            task.loc.clone(),
                            HashMap::from([(RESULT_OUTPUT.to_string(), task.output.clone())]),
                        )],
                    )
                    .await?;
                    break;
                }
                TaskStatus::Failed => {
                    let logs = resource
                        .lock()
                        .await
                        .task_logs(&task.task_id)
                        .await
                        .map_or_else(|_| String::new(), |logs| format!(": {logs}"));
                    return Err(miette!("QRMI task `{}` failed{logs}", task.task_id));
                }
                TaskStatus::Cancelled => {
                    send_cancelled(
                        &mut event_sender,
                        task.workflow_run_id,
                        task.attempt,
                        task.loc.clone(),
                    )
                    .await?;
                    break;
                }
            }
            tokio::time::sleep(poll_interval).await;
        }
        Ok(())
    }
    .await;
    (key, result)
}

async fn process_tasks(
    resource: Resource,
    mut task_receiver: TaskReceiver,
    mut cancel_receiver: CancelReceiver,
    mut event_sender: EventSender,
    asset_storage_registry: AssetStorageRegistry,
    poll_interval: Duration,
) {
    let mut registrations: HashMap<TaskKey, TaskRegistration> = HashMap::new();
    let mut running = RunningTasks::new();

    loop {
        tokio::select! {
            Some(key) = cancel_receiver.next() => {
                match registrations.remove(&key) {
                    Some(TaskRegistration::Watching(task_id)) => {
                        if let Err(err) = resource.lock().await.task_stop(&task_id).await {
                            let err = miette!("Failed to stop QRMI task `{task_id}`: {err}");
                            let _ = send_error(&mut event_sender, key.0, key.1, key.2, &err).await;
                        }
                    }
                    Some(TaskRegistration::CancelBeforeRegistration) | None => {
                        registrations.insert(key, TaskRegistration::CancelBeforeRegistration);
                    }
                }
            }
            Some(task) = task_receiver.next() => {
                let key = task.key();
                let cancel_now = matches!(
                    registrations.remove(&key),
                    Some(TaskRegistration::CancelBeforeRegistration)
                );
                if cancel_now {
                    if let Err(err) = resource.lock().await.task_stop(&task.task_id).await {
                        let err = miette!("Failed to stop QRMI task `{}`: {err}", task.task_id);
                        let _ = send_error(
                            &mut event_sender,
                            task.workflow_run_id,
                            task.attempt,
                            task.loc.clone(),
                            &err,
                        ).await;
                    }
                } else {
                    registrations.insert(key, TaskRegistration::Watching(task.task_id.clone()));
                }
                running.push(monitor_task(
                    Arc::clone(&resource),
                    event_sender.clone(),
                    Arc::clone(&asset_storage_registry),
                    poll_interval,
                    task,
                ).boxed());
            }
            Some((key, result)) = running.next() => {
                registrations.remove(&key);
                if let Err(err) = result {
                    let _ = send_error(&mut event_sender, key.0, key.1, key.2, &err).await;
                }
            }
            else => break,
        }
    }
}

/// An [`Executor`] that submits and monitors tasks through QRMI.
pub struct QrmiExecutor {
    resource: Resource,
    resource_type: QrmiResourceType,
    task_sender: TaskSender,
    cancel_sender: CancelSender,
    event_receiver: Mutex<Option<EventReceiver>>,
    background_abort_handle: AbortHandle,
    output_storage_name: String,
    asset_storage_registry: AssetStorageRegistry,
}

impl Drop for QrmiExecutor {
    fn drop(&mut self) {
        self.background_abort_handle.abort();
    }
}

impl QrmiExecutor {
    /// Construct an executor for a fixed QRMI resource.
    ///
    /// # Errors
    ///
    /// Returns an error if the output storage is unknown or QRMI cannot
    /// construct the configured resource.
    pub fn try_new(
        config: &QrmiExecutorConfig,
        asset_storage_registry: &AssetStorageRegistry,
        output_storage_name: &str,
    ) -> miette::Result<Self> {
        let resource = construct_resource(config)?;
        Self::try_new_with_resource(
            config,
            asset_storage_registry,
            output_storage_name,
            resource,
        )
    }

    fn try_new_with_resource(
        config: &QrmiExecutorConfig,
        asset_storage_registry: &AssetStorageRegistry,
        output_storage_name: &str,
        resource: Box<dyn QuantumResource>,
    ) -> miette::Result<Self> {
        if !asset_storage_registry.contains_key(output_storage_name) {
            return Err(miette!("output_storage_name not in registry"));
        }
        if config.poll_interval_secs == 0 {
            return Err(miette!("poll_interval_secs must be greater than zero"));
        }

        let resource = Arc::new(AsyncMutex::new(resource));
        let (task_sender, task_receiver) = mpsc::channel(64);
        let (cancel_sender, cancel_receiver) = mpsc::channel(64);
        let (event_sender, event_receiver) = mpsc::channel(64);
        let background_task = tokio::spawn(process_tasks(
            Arc::clone(&resource),
            task_receiver,
            cancel_receiver,
            event_sender,
            Arc::clone(asset_storage_registry),
            Duration::from_secs(config.poll_interval_secs),
        ));

        Ok(Self {
            resource,
            resource_type: config.resource_type,
            task_sender,
            cancel_sender,
            event_receiver: Mutex::new(Some(event_receiver)),
            background_abort_handle: background_task.abort_handle(),
            output_storage_name: output_storage_name.to_string(),
            asset_storage_registry: Arc::clone(asset_storage_registry),
        })
    }
}

impl Executor for QrmiExecutor {
    fn workers(&self) -> BoxFuture<'_, miette::Result<Vec<WorkerSpec>>> {
        futures::future::ok(vec![WorkerSpec {
            worker_name: WORKER_NAME.to_string(),
        }])
        .boxed()
    }

    fn execute(&self, task_plans: Vec<TaskPlan>) -> BoxFuture<'_, miette::Result<()>> {
        async move {
            let mut task_sender = self.task_sender.clone();
            for task_plan in task_plans {
                if task_plan.worker_name != WORKER_NAME {
                    return Err(miette!("Unknown worker: `{}`", task_plan.worker_name));
                }
                if task_plan.task_name != TASK_NAME {
                    return Err(miette!("Unknown task: `{}`", task_plan.task_name));
                }
                if task_plan.inputs.len() != 1 || !task_plan.inputs.contains_key(PAYLOAD_INPUT) {
                    return Err(miette!("QRMI task inputs must contain only `payload`"));
                }
                if task_plan.outputs != HashSet::from([RESULT_OUTPUT.to_string()]) {
                    return Err(miette!("QRMI task outputs must contain only `result`"));
                }

                let output_storage_name = task_plan
                    .output_storage_name
                    .as_deref()
                    .unwrap_or(&self.output_storage_name);
                let output =
                    reserve_asset_specs(&self.asset_storage_registry, output_storage_name, 1)
                        .await?
                        .into_iter()
                        .next()
                        .ok_or_else(|| miette!("Failed to reserve QRMI result asset"))?;

                let task_id = if let Some(task_id) = task_plan.task_handle {
                    task_id
                } else {
                    let payload = load_asset(
                        &self.asset_storage_registry,
                        &task_plan.inputs,
                        PAYLOAD_INPUT,
                    )
                    .await?;
                    let payload: QrmiPayload = serde_json::from_slice(&payload)
                        .into_diagnostic()
                        .wrap_err("Failed to decode QRMI payload")?;
                    payload.validate_for(self.resource_type)?;
                    self.resource
                        .lock()
                        .await
                        .task_start(payload.into())
                        .await
                        .into_diagnostic()
                        .wrap_err("Failed to start QRMI task")?
                };

                task_sender
                    .send(BackgroundTask {
                        workflow_run_id: task_plan.workflow_run_id,
                        attempt: task_plan.attempt,
                        loc: task_plan.loc,
                        task_id,
                        output,
                    })
                    .await
                    .into_diagnostic()
                    .wrap_err("Failed to register QRMI task for monitoring")?;
            }
            Ok(())
        }
        .boxed()
    }

    fn listen(&self) -> miette::Result<BoxStream<'static, RuntimeEvent>> {
        let channel = self
            .event_receiver
            .try_lock()
            .map_err(|err| miette!("Failed to listen: {err}"))?
            .take()
            .ok_or_else(|| miette!("Failed to listen: Executor is already being listened to."))?;
        Ok(channel.boxed())
    }

    fn cancel(
        &self,
        workflow_run_id: Uuid,
        attempt: u32,
        task_locations: Vec<Location>,
    ) -> BoxFuture<'_, miette::Result<()>> {
        let mut cancel_sender = self.cancel_sender.clone();
        async move {
            for loc in task_locations {
                cancel_sender
                    .send((workflow_run_id, attempt, loc))
                    .await
                    .into_diagnostic()
                    .wrap_err("Failed to request QRMI task cancellation")?;
            }
            Ok(())
        }
        .boxed()
    }
}

#[cfg(test)]
mod tests {
    use std::{
        assert_matches,
        collections::VecDeque,
        sync::{Arc, Mutex},
    };

    use async_trait::async_trait;
    use futures::StreamExt;
    use qrmi::models::{ResourceType, TaskResult};
    use serde_json::json;

    use crate::{
        asset_storage::{assert_registry_contains_values, test_storage_registry},
        event::{NodeStatus, WorkflowRunEvent},
    };

    use super::*;

    struct FakeState {
        starts: usize,
        stops: Vec<String>,
        statuses: VecDeque<TaskStatus>,
    }

    struct FakeResource {
        state: Arc<Mutex<FakeState>>,
        result: String,
    }

    #[async_trait]
    impl QuantumResource for FakeResource {
        async fn resource_id(&mut self) -> qrmi::Result<String> {
            Ok("fake-resource".to_string())
        }

        async fn resource_type(&mut self) -> qrmi::Result<ResourceType> {
            Ok(ResourceType::IBMQuantumComputeService)
        }

        async fn is_accessible(&mut self) -> qrmi::Result<bool> {
            Ok(true)
        }

        async fn task_start(&mut self, _payload: Payload) -> qrmi::Result<String> {
            self.state.lock().unwrap().starts += 1;
            Ok("fake-task-id".to_string())
        }

        async fn task_stop(&mut self, task_id: &str) -> qrmi::Result<()> {
            self.state.lock().unwrap().stops.push(task_id.to_string());
            Ok(())
        }

        async fn task_status(&mut self, _task_id: &str) -> qrmi::Result<TaskStatus> {
            let mut state = self.state.lock().unwrap();
            Ok(state.statuses.pop_front().unwrap_or(TaskStatus::Completed))
        }

        async fn task_result(&mut self, _task_id: &str) -> qrmi::Result<TaskResult> {
            Ok(TaskResult {
                value: self.result.clone(),
            })
        }
    }

    fn config() -> QrmiExecutorConfig {
        QrmiExecutorConfig {
            resource_id: "fake-resource".to_string(),
            resource_type: QrmiResourceType::IBMQuantumComputeService,
            environment: HashMap::new(),
            poll_interval_secs: 1,
        }
    }

    fn fake_resource(
        statuses: impl IntoIterator<Item = TaskStatus>,
        result: &serde_json::Value,
    ) -> (Box<dyn QuantumResource>, Arc<Mutex<FakeState>>) {
        let state = Arc::new(Mutex::new(FakeState {
            starts: 0,
            stops: vec![],
            statuses: statuses.into_iter().collect(),
        }));
        (
            Box::new(FakeResource {
                state: Arc::clone(&state),
                result: result.to_string(),
            }),
            state,
        )
    }

    fn task_plan(inputs: HashMap<String, AssetSpec>) -> TaskPlan {
        TaskPlan {
            workflow_run_id: Uuid::new_v4(),
            worker_name: WORKER_NAME.to_string(),
            task_name: TASK_NAME.to_string(),
            inputs,
            outputs: HashSet::from([RESULT_OUTPUT.to_string()]),
            ..Default::default()
        }
    }

    #[tokio::test]
    async fn execute_emits_events_and_saves_result() -> miette::Result<()> {
        let (registry, input_sets, _dir) = test_storage_registry(
            vec![json!({
                "payload": {
                    "type": "qiskit_primitive",
                    "input": "{}",
                    "program_id": "sampler"
                }
            })],
            vec![],
        )
        .await;
        let (resource, state) = fake_resource(
            [TaskStatus::Running, TaskStatus::Completed],
            &json!({"counts": {"0": 10}}),
        );
        let executor =
            QrmiExecutor::try_new_with_resource(&config(), &registry, "memory", resource)?;
        let stream = executor.listen()?;

        executor
            .execute(vec![task_plan(input_sets[0].clone())])
            .await?;
        let events = stream.take(3).collect::<Vec<_>>().await;

        assert_eq!(events.len(), 3);
        assert_matches!(
            events[0],
            RuntimeEvent::WorkflowRun {
                event: WorkflowRunEvent::NodeEvents(ref node_events),
                ..
            } if matches!(node_events[0].status, NodeStatus::Queued { ref handle }
                if handle.as_deref() == Some("fake-task-id"))
        );
        assert_matches!(
            events[1],
            RuntimeEvent::WorkflowRun {
                event: WorkflowRunEvent::NodeEvents(ref node_events),
                ..
            } if matches!(node_events[0].status, NodeStatus::Running { .. })
        );
        assert_matches!(
            events[2],
            RuntimeEvent::WorkflowRun {
                event: WorkflowRunEvent::NodeEvents(ref node_events),
                ..
            } if matches!(node_events[0].status, NodeStatus::Complete { .. })
        );
        assert_registry_contains_values(
            &registry,
            "memory",
            &events[2].clone().outputs()[0],
            json!({"result": {"counts": {"0": 10}}}),
        )
        .await;
        assert_eq!(state.lock().unwrap().starts, 1);
        Ok(())
    }

    #[tokio::test]
    async fn restored_task_is_not_resubmitted() -> miette::Result<()> {
        let (registry, input_sets, _dir) = test_storage_registry(
            vec![json!({
                "payload": {
                    "type": "qiskit_primitive",
                    "input": "{}",
                    "program_id": "sampler"
                }
            })],
            vec![],
        )
        .await;
        let (resource, state) = fake_resource([TaskStatus::Completed], &json!({"done": true}));
        let executor =
            QrmiExecutor::try_new_with_resource(&config(), &registry, "memory", resource)?;
        let stream = executor.listen()?;
        let mut plan = task_plan(input_sets[0].clone());
        plan.task_handle = Some("restored-task-id".to_string());

        executor.execute(vec![plan]).await?;
        let events = stream.take(2).collect::<Vec<_>>().await;

        assert_eq!(events.len(), 2);
        assert_matches!(
            events[0],
            RuntimeEvent::WorkflowRun {
                event: WorkflowRunEvent::NodeEvents(ref node_events),
                ..
            } if matches!(node_events[0].status, NodeStatus::Queued { ref handle }
                if handle.as_deref() == Some("restored-task-id"))
        );
        assert_eq!(state.lock().unwrap().starts, 0);
        Ok(())
    }

    #[tokio::test]
    async fn cancel_stops_registered_task() -> miette::Result<()> {
        let (registry, input_sets, _dir) = test_storage_registry(
            vec![json!({
                "payload": {
                    "type": "qiskit_primitive",
                    "input": "{}",
                    "program_id": "sampler"
                }
            })],
            vec![],
        )
        .await;
        let (resource, state) = fake_resource([TaskStatus::Queued], &json!(null));
        let executor =
            QrmiExecutor::try_new_with_resource(&config(), &registry, "memory", resource)?;
        let mut stream = executor.listen()?;
        let plan = task_plan(input_sets[0].clone());
        let run_id = plan.workflow_run_id;
        let loc = plan.loc.clone();

        executor.execute(vec![plan]).await?;
        let _queued = stream.next().await;
        executor.cancel(run_id, 0, vec![loc]).await?;

        tokio::time::timeout(Duration::from_secs(1), async {
            loop {
                if !state.lock().unwrap().stops.is_empty() {
                    break;
                }
                tokio::task::yield_now().await;
            }
        })
        .await
        .into_diagnostic()?;
        assert_eq!(state.lock().unwrap().stops, ["fake-task-id"]);
        Ok(())
    }

    #[test]
    fn payloads_are_validated_against_resource_type() -> miette::Result<()> {
        let payload: QrmiPayload = serde_json::from_value(json!({
            "type": "iqm_server",
            "iqmjson": "{}",
            "job_type": "circuit",
            "use_timeslot": null,
            "tag": null
        }))
        .into_diagnostic()?;

        payload.validate_for(QrmiResourceType::IQMServer)?;
        assert!(
            payload
                .validate_for(QrmiResourceType::IBMQuantumComputeService)
                .is_err()
        );
        Ok(())
    }

    #[test]
    fn config_uses_qrmi_resource_names_and_poll_default() -> miette::Result<()> {
        let config: QrmiExecutorConfig = serde_json::from_value(json!({
            "resource_id": "ibm_torino",
            "resource_type": "ibm-quantum-compute-service"
        }))
        .into_diagnostic()?;

        assert_eq!(config.poll_interval_secs, 1);
        assert!(config.environment.is_empty());
        assert_eq!(
            config.resource_type,
            QrmiResourceType::IBMQuantumComputeService
        );
        Ok(())
    }
}
