# Core concepts in Tierkreis

Tierkreis represents a computation as a workflow graph. Tasks are nodes and
edges carry their inputs and outputs. Data dependencies determine when a task
is ready, so independent tasks can run concurrently on CPUs, GPUs, QPUs, or
remote services.

## Workflows and values

The Python graph builder catches incompatible edges before execution. Values
must also be serializable because executors and asset storage may live in
different processes or systems. See [complex types](../worker/complex_types.md)
for the supported serialization model.

Eval, Loop, Map, and IfElse nodes compose graphs into larger workflows. Nested
subgraphs and their inputs are loaded lazily: the runtime materializes them
when the containing node becomes runnable. In particular, an IfElse node does
not load or schedule its unselected branch.

## Runtime

The Rust runtime owns orchestration and persistent workflow state. A workflow
definition is saved once, and each execution is identified by a run UUID and
attempt number.

```python
from tierkreis.runtime import new_default

runtime = await new_default()
with runtime:
    workflow_id = await runtime.save_workflow("example", workflow)
    run_id = await runtime.start_new_run(workflow_id, inputs)
    await runtime.wait_for(run_id)
    outputs = await runtime.get_outputs(run_id)
```

`new_default()` loads the discovered `tierkreis.toml`, or creates the default
configuration. `new_in_memory()` is useful for builtin-only tests, while
`new_sqlite_memory()` keeps SQL state without writing a database file.

`get_outputs()` decodes JSON-compatible results into Python values. For binary
formats, `get_raw_outputs()` returns a dictionary mapping output names to their
serialized bytes so that the owning library can deserialize them.

## State and assets

Runtime state records workflows, runs, attempts, nodes, and asset keys. Asset
storage holds the serialized graph inputs and task outputs. The default
configuration uses in-memory state and file-backed assets; SQLite state and
memory assets are also available. Keeping both behind interfaces allows the
runtime to use other databases and object stores without changing workflows.

Persistent runtimes recover active work after the process is restarted. A user
can also create a new attempt with `restart_task`; completed data outside the
invalidated dependency chain is retained.

## Workers and executors

A worker implements atomic tasks. Installed workers expose commands named
`tkr-<worker-name>-worker`, which the subprocess executor discovers and
launches. Executors translate a runnable task into local, remote, or scheduled
work and transfer its inputs and outputs through asset storage.

The runtime supports in-memory and subprocess execution, Nexus and QRMI, and
PBS, PJSUB, and Slurm schedulers. Named executors, asset stores, resource
profiles, and worker commands are selected in `tierkreis.toml` and through
graph execution metadata rather than by passing Python executor objects to a
run function.
