use std::sync::{Arc, Mutex};

use futures::{Stream, StreamExt, channel::mpsc, stream::BoxStream, stream::select_all};
use miette::{Context, miette};
use tracing::instrument;
use uuid::Uuid;

use crate::{
    event::{
        EventReceiver, EventSender, RuntimeEvent, send_complete, send_map_elem_complete,
        send_running_loop, send_running_map, send_running_switching, send_workflow_run_complete,
        send_workflow_run_errored,
    },
    executor::{ExecutorRegistry, interface::TaskPlan},
};

use super::ActionPlan;

/// Performs aggregated action plans and owns executor event streams.
pub struct ActionRunner {
    event_sender: EventSender,
    event_receiver: Mutex<Option<EventReceiver>>,
    default_executor_name: String,
    executor_registry: ExecutorRegistry,
    default_storage_name: String,
}

impl ActionRunner {
    /// Create a runner backed by the supplied executor registry.
    ///
    /// # Errors
    ///
    /// Returns an error when the configured default executor is not present.
    pub fn try_new(
        executor_registry: &ExecutorRegistry,
        default_storage_name: &str,
        default_executor_name: &str,
    ) -> miette::Result<Self> {
        if !executor_registry.contains_key(default_executor_name) {
            return Err(miette!("default_executor_name not in registry"));
        }

        let (event_sender, event_receiver) = mpsc::channel(128);
        Ok(Self {
            event_sender,
            event_receiver: Mutex::new(Some(event_receiver)),
            default_executor_name: default_executor_name.to_string(),
            executor_registry: Arc::clone(executor_registry),
            default_storage_name: default_storage_name.to_string(),
        })
    }

    /// Perform an aggregated plan, dispatching tasks to an executor.
    ///
    /// Successful dispatch means that the executor accepted responsibility for
    /// the task. Resource availability and queuing remain executor concerns.
    ///
    /// # Errors
    ///
    /// Will return an error if state events cannot be emitted or an executor
    /// cannot accept the planned tasks.
    #[instrument(skip(self, plan), fields(run_id = %workflow_run_id, attempt), err)]
    pub async fn perform_plan(
        &self,
        workflow_run_id: Uuid,
        attempt: u32,
        plan: ActionPlan,
    ) -> miette::Result<()> {
        let mut event_sender = self.event_sender.clone();

        if !plan.node_complete.is_empty() {
            let (locs, outputs) = plan.node_complete.into_iter().unzip();
            send_complete(&mut event_sender, workflow_run_id, attempt, locs, outputs).await?;
        }

        for (loc, size) in plan.mapping {
            send_running_map(&mut event_sender, workflow_run_id, attempt, loc, size).await?;
        }
        for (loc, bits) in plan.map_elem_complete {
            send_map_elem_complete(&mut event_sender, workflow_run_id, attempt, loc, bits).await?;
        }
        for (loc, index) in plan.looping {
            send_running_loop(&mut event_sender, workflow_run_id, attempt, loc, index).await?;
        }
        for (loc, cond) in plan.switching {
            send_running_switching(&mut event_sender, workflow_run_id, attempt, loc, cond).await?;
        }

        let default_executor_name = &self.default_executor_name;
        let executor = self
            .executor_registry
            .get(default_executor_name)
            .ok_or_else(|| {
                miette!(
                    "Could not find an executor with name '{default_executor_name}' in ExecutorRegistry"
                )
            })
            .wrap_err("Could not run Task Nodes")?;
        let task_plans = plan
            .tasks
            .into_iter()
            .map(|task| TaskPlan {
                workflow_run_id,
                attempt,
                loc: task.loc,
                worker_name: task.worker_name,
                task_name: task.task_name,
                inputs: task.inputs,
                outputs: task.outputs,
                output_storage_name: Some(self.default_storage_name.clone()),
                resources: task.resources,
                task_handle: task.task_handle,
                ..Default::default()
            })
            .collect();
        executor
            .execute(task_plans)
            .await
            .wrap_err_with(|| miette!("Could not run Task Nodes"))?;

        if plan.workflow_error {
            send_workflow_run_errored(&mut event_sender, workflow_run_id, attempt).await?;
        }
        if plan.workflow_complete {
            send_workflow_run_complete(&mut event_sender, workflow_run_id, attempt).await?;
        }

        Ok(())
    }

    /// Listen to a combined stream of events from the runner and executors.
    ///
    /// # Errors
    ///
    /// Will return an error if this method has already been called.
    pub fn listen(&self) -> miette::Result<impl Stream<Item = RuntimeEvent> + use<>> {
        let runner_events = {
            let mut receiver = self
                .event_receiver
                .try_lock()
                .map_err(|err| miette!("Failed to listen: {err}"))?;

            receiver
                .take()
                .ok_or_else(|| miette!("Failed to listen: runner is already being listened to."))?
        };

        let mut streams = self
            .executor_registry
            .values()
            .map(|executor| executor.listen())
            .collect::<miette::Result<Vec<BoxStream<RuntimeEvent>>>>()?;

        streams.push(runner_events.boxed());
        Ok(select_all(streams))
    }
}
