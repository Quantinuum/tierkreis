use std::{
    collections::HashMap,
    hash::{DefaultHasher, Hash, Hasher},
    hint::black_box,
    time::{Duration, Instant},
};

use portgraph::NodeIndex;
use tierkreis::{
    asset_storage::{AssetData, AssetKey, AssetStorage, InMemoryStorage},
    graph::WorkflowGraph,
    location::Location,
    orchestrator::OrchestrationContext,
    state::{InMemoryRuntimeState, RuntimeState},
};

fn time(iterations: usize, mut operation: impl FnMut()) -> Duration {
    let started = Instant::now();
    for _ in 0..iterations {
        operation();
    }
    started.elapsed()
}

fn main() {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("build benchmark runtime");

    let location = Location::from_usize_iter([1, 2, 3, 4]);
    let location_time = time(1_000_000, || {
        let extended = black_box(&location).with_node(NodeIndex::new(5));
        let mut hasher = DefaultHasher::new();
        extended.hash(&mut hasher);
        black_box(hasher.finish());
    });

    let storage = InMemoryStorage::new();
    let key = AssetKey::new();
    runtime
        .block_on(storage.save(&key, AssetData::from(vec![0_u8; 1024 * 1024])))
        .expect("save benchmark asset");
    let asset_load_time = time(100_000, || {
        black_box(runtime.block_on(storage.load(&key)).expect("load asset"));
    });

    let state = InMemoryRuntimeState::new();
    let workflow_id = runtime
        .block_on(state.save_workflow(None, WorkflowGraph::new(["value".to_string()])))
        .expect("save workflow");
    let workflow_load_time = time(100_000, || {
        black_box(
            runtime
                .block_on(state.load_workflow(workflow_id))
                .expect("load workflow"),
        );
    });

    let workflow_state = runtime
        .block_on(state.new_workflow_run_state(workflow_id, HashMap::new()))
        .expect("create workflow state");
    let context = OrchestrationContext::new(&workflow_state, HashMap::new());
    let context_clone_time = time(1_000_000, || {
        black_box(context.clone());
    });

    println!("location extend/hash (1,000,000): {location_time:?}");
    println!("1 MiB in-memory load (100,000): {asset_load_time:?}");
    println!("shared workflow load (100,000): {workflow_load_time:?}");
    println!("context clone (1,000,000): {context_clone_time:?}");
}
