# Logging, errors, and restarting workflows

## Static errors

The graph builder's `TKR` types let a Python type checker catch incompatible
edges before expensive work is submitted. Runtime errors can still occur when
a worker raises an exception, exits unsuccessfully, or returns invalid data.

## Runtime diagnostics

`wait_for` raises `ValueError` when a workflow attempt fails. The exception
includes related diagnostics for the failed node. The run remains available
for inspection:

```python
try:
    await runtime.wait_for(run_id)
except ValueError as error:
    print(error)
    summary = await runtime.get_summary(run_id)
    print(summary.errored_locations)
```

`get_summary(run_id, attempt)` reports timestamps, duration, errored locations,
and locations paused at breakpoints. For lower-level debugging,
`debug_read_node_states(run_id, attempt, locations)` returns the state, task
name, and execution context for the requested location strings.

Set `-v` on `tkr run`, or call `config.set_log_level("debug")` before
`new_from_config(config)`, to enable runtime debug logging. Worker logs remain
owned by the selected executor or remote service.

## Restarting failed or selected work

Restarting creates a new attempt and leaves the source attempt unchanged. With
no locations, every errored node is restarted. Supplying locations restarts
those nodes and transitively invalidates their dependent results:

```python
new_attempt, invalidated = await runtime.restart_task(run_id)
await runtime.wait_for(run_id, new_attempt)
outputs = await runtime.get_outputs(run_id, new_attempt)
```

```python
new_attempt, invalidated = await runtime.restart_task(run_id, ["N3"])
```

Locations may identify nodes nested inside Eval, Loop, or Map. Their containing
nodes are partially invalidated as needed, while unaffected work is reused.
See the [restart example](../examples/restart.ipynb) for a complete workflow.

## Pausing and resuming at breakpoints

Set `has_breakpoint=True` when adding a node to pause it immediately before
execution. `wait_for` returns when the run pauses, allowing the run state to be
inspected. The summary contains the exact paused locations:

```python
task_output = graph.task(worker_task(...), has_breakpoint=True)

await runtime.wait_for(run_id)
summary = await runtime.get_summary(run_id)
print(summary.paused_locations)
```

Calling `unpause_nodes` without locations resumes every currently paused node.
Pass location strings to resume only selected nodes. After resuming, call
`wait_for` again; it returns at the next breakpoint or when execution finishes:

```python
unpaused = await runtime.unpause_nodes(run_id)
await runtime.wait_for(run_id)

# Or resume selected nodes in a particular attempt.
unpaused = await runtime.unpause_nodes(run_id, ["N3"], attempt=1)
```

Input and output nodes can also be breakpoints by constructing `Graph` with
`breakpoints_on_inputs=True` or `breakpoints_on_outputs=True`.

## Recovering after the runtime stops

With persistent SQLite state, starting another runtime over the same
configuration recovers active runs and resumes orchestration. Keep the same
state database, asset storage, and worker configuration. This process recovery
is distinct from `restart_task`: recovery continues the current attempt,
whereas a task restart creates a new attempt after a failure or manual choice.

The legacy interactive `debug_graph` helper is not part of the Rust runtime.
Use breakpoints together with run summaries and `debug_read_node_states` to
build an interactive debugging flow.
