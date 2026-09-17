use std::collections::HashMap;

use bitvec::vec::BitVec;
use futures::{Stream, StreamExt};
use miette::miette;
use tracing::{debug, instrument};

use crate::location::Location;

use super::{
    Action, ActionKind, ActionPlan, ExecutorTaskGroup, LoopUpdate, MapElementsCompleted, MapStart,
    NodeCompletion, PlannedTask, SwitchUpdate, WorkflowOutcome,
};

/// Aggregates an action stream into an inspectable [`ActionPlan`].
#[derive(Debug, Clone)]
pub struct ActionAggregator {
    default_executor_name: String,
}

impl ActionAggregator {
    /// Create an aggregator that assigns new tasks to the named executor.
    ///
    /// Executor assignment is deliberately explicit in [`ActionPlan`] even
    /// while the policy is limited to one configured default.
    pub fn new(default_executor_name: impl Into<String>) -> Self {
        Self {
            default_executor_name: default_executor_name.into(),
        }
    }

    /// Consume an action stream and produce a debuggable execution plan.
    ///
    /// # Errors
    ///
    /// Returns the first planning error yielded by the action stream or an
    /// error when the actions describe an inconsistent plan.
    #[instrument(skip(self, actions), err)]
    pub async fn aggregate(
        &self,
        mut actions: impl Stream<Item = miette::Result<Action>> + Unpin,
    ) -> miette::Result<ActionPlan> {
        let mut plan = ActionPlan::default();
        let mut map_completion_indices = HashMap::<Location, usize>::new();
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
                } => self
                    .tasks_for_default_executor(&mut plan)
                    .push(PlannedTask {
                        loc,
                        worker_name,
                        task_name,
                        inputs,
                        outputs,
                        task_handle,
                        resources,
                    }),
                ActionKind::SetSwitching { cond } => {
                    plan.node_updates.switches.push(SwitchUpdate {
                        location: loc,
                        condition: cond,
                    });
                }
                ActionKind::SetRunningLoop { index } => {
                    plan.node_updates.loops.push(LoopUpdate {
                        location: loc,
                        index,
                    });
                }
                ActionKind::SetRunningMap { size } => {
                    plan.node_updates.maps_started.push(MapStart {
                        location: loc,
                        size,
                    });
                }
                ActionKind::SetMapElemComplete { index, size } => {
                    if index >= size {
                        return Err(miette!(
                            "map element index {index} is outside map of size {size} at {loc}"
                        ));
                    }

                    let update_index = if let Some(update_index) = map_completion_indices.get(&loc)
                    {
                        *update_index
                    } else {
                        let update_index = plan.node_updates.map_elements_completed.len();
                        plan.node_updates
                            .map_elements_completed
                            .push(MapElementsCompleted {
                                location: loc.clone(),
                                completed: BitVec::repeat(false, size),
                            });
                        map_completion_indices.insert(loc.clone(), update_index);
                        update_index
                    };
                    let update = &mut plan.node_updates.map_elements_completed[update_index];
                    if update.completed.len() != size {
                        return Err(miette!(
                            "conflicting map sizes at {loc}: {} and {size}",
                            update.completed.len()
                        ));
                    }
                    update.completed.set(index, true);
                }
                ActionKind::SetComplete { outputs } => {
                    plan.node_updates.completions.push(NodeCompletion {
                        location: loc,
                        outputs,
                    });
                }
                ActionKind::WorkflowErrored {} => {
                    set_workflow_outcome(&mut plan, WorkflowOutcome::Errored)?;
                }
                ActionKind::WorkflowFinished {} => {
                    set_workflow_outcome(&mut plan, WorkflowOutcome::Completed)?;
                }
            }
        }

        debug!(summary = ?plan.summary(), "Built action plan");
        Ok(plan)
    }

    fn tasks_for_default_executor<'a>(&self, plan: &'a mut ActionPlan) -> &'a mut Vec<PlannedTask> {
        if let Some(group_index) = plan
            .executor_groups
            .iter()
            .position(|group| group.executor_name == self.default_executor_name)
        {
            return &mut plan.executor_groups[group_index].tasks;
        }

        let group_index = plan.executor_groups.len();
        plan.executor_groups.push(ExecutorTaskGroup {
            executor_name: self.default_executor_name.clone(),
            tasks: Vec::new(),
        });
        &mut plan.executor_groups[group_index].tasks
    }
}

fn set_workflow_outcome(plan: &mut ActionPlan, outcome: WorkflowOutcome) -> miette::Result<()> {
    match plan.outcome {
        None => plan.outcome = Some(outcome),
        Some(current) if current == outcome => {}
        Some(current) => {
            return Err(miette!(
                "conflicting workflow outcomes in one action plan: {current:?} and {outcome:?}"
            ));
        }
    }
    Ok(())
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

        let plan = ActionAggregator::new("gpu")
            .aggregate(stream::iter([Ok(action)]))
            .await?;

        assert_eq!(plan.executor_groups().len(), 1);
        assert_eq!(plan.executor_groups()[0].executor_name, "gpu");
        assert_eq!(plan.executor_groups()[0].tasks.len(), 1);
        assert_eq!(plan.executor_groups()[0].tasks[0].resources(), &resources);
        Ok(())
    }

    #[tokio::test]
    async fn groups_tasks_by_default_executor_assignment() -> miette::Result<()> {
        let actions = ["N1", "N2"].map(|location| {
            Ok(Action {
                loc: Location::new(location)?,
                kind: ActionKind::PerformTask {
                    worker_name: "worker".to_string(),
                    task_name: "task".to_string(),
                    inputs: HashMap::new(),
                    outputs: HashSet::new(),
                    task_handle: None,
                    resources: HashMap::new(),
                },
            })
        });

        let plan = ActionAggregator::new("batch")
            .aggregate(stream::iter(actions))
            .await?;

        assert_eq!(plan.executor_groups().len(), 1);
        assert_eq!(plan.executor_groups()[0].executor_name, "batch");
        assert_eq!(plan.executor_groups()[0].tasks.len(), 2);
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

        let plan = ActionAggregator::new("memory")
            .aggregate(stream::iter(actions))
            .await?;

        let update = &plan.node_updates().map_elements_completed[0];
        assert_eq!(update.location, loc);
        let completed = &update.completed;
        assert_eq!(completed.len(), 3);
        assert!(completed[0]);
        assert!(!completed[1]);
        assert!(completed[2]);
        Ok(())
    }

    #[tokio::test]
    async fn rejects_conflicting_workflow_outcomes() -> miette::Result<()> {
        let actions = [
            Ok(Action {
                loc: Location::root(),
                kind: ActionKind::WorkflowFinished {},
            }),
            Ok(Action {
                loc: Location::root(),
                kind: ActionKind::WorkflowErrored {},
            }),
        ];

        let error = ActionAggregator::new("memory")
            .aggregate(stream::iter(actions))
            .await
            .unwrap_err();

        assert!(error.to_string().contains("conflicting workflow outcomes"));
        Ok(())
    }

    #[tokio::test]
    async fn rejects_conflicting_map_sizes() -> miette::Result<()> {
        let loc = Location::new("N1")?;
        let actions = [(0, 2), (1, 3)].map(|(index, size)| {
            Ok(Action {
                loc: loc.clone(),
                kind: ActionKind::SetMapElemComplete { index, size },
            })
        });

        let error = ActionAggregator::new("memory")
            .aggregate(stream::iter(actions))
            .await
            .unwrap_err();

        assert!(error.to_string().contains("conflicting map sizes"));
        Ok(())
    }
}
