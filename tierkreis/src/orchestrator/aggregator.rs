use bitvec::vec::BitVec;
use futures::{Stream, StreamExt};
use tracing::{debug, instrument};

use super::{Action, ActionKind, ActionPlan, PlannedTask};

/// Aggregates an action stream into an inspectable [`ActionPlan`].
#[derive(Debug, Clone, Copy, Default)]
pub struct ActionAggregator;

impl ActionAggregator {
    /// Consume an action stream and produce a debuggable execution plan.
    ///
    /// # Errors
    ///
    /// Returns the first planning error yielded by the action stream.
    #[instrument(skip(self, actions), err)]
    pub async fn aggregate(
        &self,
        mut actions: impl Stream<Item = miette::Result<Action>> + Unpin,
    ) -> miette::Result<ActionPlan> {
        let mut plan = ActionPlan::default();
        while let Some(Action { loc, kind }) = actions.next().await.transpose()? {
            debug!("Aggregating action at {loc}, kind: {kind:?}");
            match kind {
                ActionKind::PerformTask {
                    worker_name,
                    task_name,
                    inputs,
                    outputs,
                    task_handle,
                    resources,
                } => plan.tasks.push(PlannedTask {
                    loc,
                    worker_name,
                    task_name,
                    inputs,
                    outputs,
                    task_handle,
                    resources,
                }),
                ActionKind::SetSwitching { cond } => plan.switching.push((loc, cond)),
                ActionKind::SetRunningLoop { index } => plan.looping.push((loc, index)),
                ActionKind::SetRunningMap { size } => plan.mapping.push((loc, size)),
                ActionKind::SetMapElemComplete { index, size } => {
                    let entry = plan
                        .map_elem_complete
                        .entry(loc)
                        .or_insert_with(|| BitVec::repeat(false, size));
                    entry.set(index, true);
                }
                ActionKind::SetComplete { outputs } => {
                    plan.node_complete.push((loc, outputs));
                }
                ActionKind::WorkflowErrored {} => plan.workflow_error = true,
                ActionKind::WorkflowFinished {} => plan.workflow_complete = true,
            }
        }

        Ok(plan)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::{HashMap, HashSet};

    use futures::stream;
    use serde_json::json;

    use crate::location::Location;

    use super::*;

    #[tokio::test]
    async fn preserves_executor_owned_resource_requests() -> miette::Result<()> {
        let resources = HashMap::from([
            ("nodes".to_string(), json!(1)),
            ("gpus_per_node".to_string(), json!(2)),
        ]);
        let action = Action {
            loc: Location::new("N1")?,
            kind: ActionKind::PerformTask {
                worker_name: "gpu_worker".to_string(),
                task_name: "train".to_string(),
                inputs: HashMap::new(),
                outputs: HashSet::new(),
                task_handle: None,
                resources: resources.clone(),
            },
        };

        let plan = ActionAggregator
            .aggregate(stream::iter([Ok(action)]))
            .await?;

        assert_eq!(plan.tasks().len(), 1);
        assert_eq!(plan.tasks()[0].resources(), &resources);
        Ok(())
    }

    #[tokio::test]
    async fn coalesces_map_element_updates() -> miette::Result<()> {
        let loc = Location::new("N1")?;
        let actions = [0, 2].map(|index| {
            Ok(Action {
                loc: loc.clone(),
                kind: ActionKind::SetMapElemComplete { index, size: 3 },
            })
        });

        let plan = ActionAggregator.aggregate(stream::iter(actions)).await?;

        let completed = &plan.map_elem_complete[&loc];
        assert_eq!(completed.len(), 3);
        assert!(completed[0]);
        assert!(!completed[1]);
        assert!(completed[2]);
        Ok(())
    }
}
