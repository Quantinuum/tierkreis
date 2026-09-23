/*!
Synthetic demo data generation, used to populate the monitoring dashboard
with realistic-looking Workflows/runs/attempts without external dependencies.
*/
use std::{collections::HashMap, sync::Arc};

use miette::IntoDiagnostic;
use uuid::Uuid;

use crate::{
    builder::{constant, input, link, output, task, workflow},
    graph::WorkflowGraph,
    runtime::{Runtime, RuntimeConfig},
};

/// CLI entry point: build a persistent [`Runtime`] (the same configuration used by
/// `tkr serve`) and seed it with synthetic demo data.
///
/// Uses the same config as `tkr serve` (rather than a purely in-memory one) so
/// that seeded runs can still be restarted/inspected by a separately-running
/// `tkr serve` process afterwards - in-memory asset storage is not shared
/// across processes.
///
/// # Errors
///
/// Will return Err if the [`Runtime`] cannot be constructed or seeding fails.
///
/// # Panics
///
/// Will panic if the built-in demo Workflow graphs fail to serialize or link,
/// which should be impossible.
#[tokio::main]
pub async fn seed() -> miette::Result<()> {
    let runtime = Arc::new(Runtime::from_config(&RuntimeConfig::persistent()).await?);
    seed_demo_data(&runtime).await
}

/// Populate the runtime's state with a handful of synthetic Workflows, runs
/// and attempts, so the monitoring dashboard has data to show.
///
/// Uses only the built-in in-memory executor, so this works regardless of
/// which real Workers are installed.
///
/// # Errors
///
/// Will return Err if a Workflow or run cannot be created.
///
/// # Panics
///
/// Will panic if the built-in demo Workflow graphs fail to serialize or link,
/// which should be impossible.
pub async fn seed_demo_data(runtime: &Arc<Runtime>) -> miette::Result<()> {
    let mut pending: Vec<(Uuid, u32)> = Vec::new();

    // Workflow A: a simple addition pipeline that always succeeds.
    let addition_id = runtime
        .save_workflow(Some("addition-pipeline".to_string()), addition_graph())
        .await?;
    for value in [5, 12] {
        let inputs = HashMap::from([(
            "value".to_string(),
            serde_json::to_vec(&value).into_diagnostic()?,
        )]);
        pending.push(runtime.start_new_run(addition_id, inputs).await?);
    }

    // Workflow B: references a task that doesn't exist, so every attempt errors.
    // Demonstrates a run with multiple (failing) attempts.
    let broken_id = runtime
        .save_workflow(Some("unstable-pipeline".to_string()), broken_graph())
        .await?;
    let inputs = HashMap::from([(
        "value".to_string(),
        serde_json::to_vec(&7).into_diagnostic()?,
    )]);
    let (broken_run_id, broken_attempt) = runtime.start_new_run(broken_id, inputs).await?;
    pending.push((broken_run_id, broken_attempt));

    drive_and_wait(runtime, &pending).await;

    // Restart the failed run to create a second attempt (fails again, same graph).
    let second_attempt = runtime.create_next_attempt(broken_run_id).await?;
    drive_and_wait(runtime, &[(broken_run_id, second_attempt)]).await;

    // Workflow C: saved but never run, to demonstrate Workflows with no run
    // history yet still show up in the monitor.
    runtime
        .save_workflow(Some("draft-pipeline".to_string()), addition_graph())
        .await?;

    // Workflow D: takes two named inputs, to exercise the dynamic multi-field new-run form.
    let sum_id = runtime
        .save_workflow(Some("sum-pipeline".to_string()), sum_graph())
        .await?;
    let inputs = HashMap::from([
        ("a".to_string(), serde_json::to_vec(&3).into_diagnostic()?),
        ("b".to_string(), serde_json::to_vec(&4).into_diagnostic()?),
    ]);
    let pending = vec![runtime.start_new_run(sum_id, inputs).await?];
    drive_and_wait(runtime, &pending).await;

    tracing::info!("Seeded demo data: 4 workflows, 4 runs, 5 attempts");
    Ok(())
}

/// Drive the orchestrator until `pending` runs finish, ignoring errors from
/// runs that are expected to fail.
async fn drive_and_wait(runtime: &Arc<Runtime>, pending: &[(Uuid, u32)]) {
    let orchestration_runtime = Arc::clone(runtime);
    let orchestration = orchestration_runtime.run();
    let waits = async {
        for (run_id, attempt) in pending {
            let _ = runtime.wait_for(*run_id, *attempt).await;
        }
    };
    // Stop as soon as all pending runs finish; the orchestration loop itself
    // never returns on its own, so it's dropped once `waits` completes.
    tokio::select! {
        _ = orchestration => {},
        () = waits => {},
    }
}

fn addition_graph() -> WorkflowGraph {
    let mut wf = workflow(["result"]);
    let value = input(&mut wf, "value");
    let one = constant(&mut wf, 1).expect("failed to serialize constant");
    let two = constant(&mut wf, 2).expect("failed to serialize constant");

    let plus_one = task(&mut wf, "builtins", "iadd", ["a", "b"], ["value"]);
    link(&mut wf, one, (plus_one, "a")).expect("failed to link");
    link(&mut wf, value, (plus_one, "b")).expect("failed to link");

    let doubled = task(&mut wf, "builtins", "itimes", ["a", "b"], ["value"]);
    link(&mut wf, two, (doubled, "a")).expect("failed to link");
    link(&mut wf, (plus_one, "value"), (doubled, "b")).expect("failed to link");

    let out = output(&wf, "result");
    link(&mut wf, (doubled, "value"), out).expect("failed to link");

    wf
}

fn broken_graph() -> WorkflowGraph {
    let mut wf = workflow(["result"]);
    let value = input(&mut wf, "value");
    // "backflip" is not a real task, so this always errors.
    let broken = task(&mut wf, "builtins", "backflip", ["value"], ["value"]);
    link(&mut wf, value, (broken, "value")).expect("failed to link");
    let out = output(&wf, "result");
    link(&mut wf, (broken, "value"), out).expect("failed to link");
    wf
}

fn sum_graph() -> WorkflowGraph {
    let mut wf = workflow(["result"]);
    let a = input(&mut wf, "a");
    let b = input(&mut wf, "b");
    let sum = task(&mut wf, "builtins", "iadd", ["a", "b"], ["value"]);
    link(&mut wf, a, (sum, "a")).expect("failed to link");
    link(&mut wf, b, (sum, "b")).expect("failed to link");
    let out = output(&wf, "result");
    link(&mut wf, (sum, "value"), out).expect("failed to link");
    wf
}
