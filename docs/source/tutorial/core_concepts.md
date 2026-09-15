# Core Concepts in Tierkreis

This documents covers the basics of tierkreis.
If you want to immediately write you first graph, you can immediately skip to the [next page](../examples/first_graph.ipynb).

## Program model

In Tierkreis, a computation is represented as a sequence of tasks comprising a workflow.
They are combined in a directed acyclic graph (DAG), where each task is represented by a node.
In this documentation we use the terms graph and workflow interchangeably.
An edge indicates that data computed by one node is required to start another node.
The data dependencies induce a partial order of execution, which allows parallel and asynchronous execution.
As a result a computation can be distributed easily and run on different types of hardware including remote (cloud) and onsite:

- CPUs
- GPUs
- QPUs

### Tasks

Tasks are the basic building blocks of a workflow.
They are independent and loosely coupled and represent an atomic operation in the workflow.
Still, they can consume large amounts of resources and running time.
While they can maintain state for the duration of their lifetime in the context of the workflow they are stateless.
They simply act on their inputs and produce outputs, which are linked to other tasks.

### Workflows

Workflows are a composition of tasks, that provide the control structure and data flow.
Higher-order constructions like nesting graphs, folding and mapping can be used to create complex workflows.
From the data dependencies, the runtime environment can infer which tasks it needs to run and can do so in parallel manner.

### Types

In Tierkreis, it is possible to associate types with edges (data) at construction.
Type information can be used while building the `Graph` to detect and prevent runtime errors.
Tierkreis values correspond to the edges in the graph.
These values can have a type assigned to them at construction time.
The set of available types is a subset of all python types, e.g. we require serialization; see more in [complex types](../worker/complex_types.md) how to add your own serialization.
As result we use the `TKR` container to promote the python types into Tierkreis compatible types.

## Execution model

The second part of Tierkreis is the execution model.
It is responsible for the orchestration and the proper execution of the tasks specified in a workflow.
The goal is to distribute tasks over the available resources, based on the capabilities of each resource.

### Runtime

The Rust-backed runtime is responsible for the execution of the entire workflow, maintaining global state and checking progress.
It checks the state of the individual tasks and decides what to run next based on the given data availability.
It combines orchestration, state storage, asset storage, and executors behind a single interface.

```python
from tierkreis import new_default

runtime = await new_default()
with runtime:
    workflow_id = await runtime.save_workflow("example", workflow)
    run_id = await runtime.start_new_run(workflow_id, inputs)
    await runtime.wait_for(run_id, 0)
    outputs = await runtime.get_outputs(run_id, 0)
```

`new_default()` adds file-backed assets and finds installed `tkr-*-worker`
commands on `PATH`. `new_in_memory()` is suitable for workflows limited to the
smaller native in-memory task set.

### Storage

The runtime stores workflow and run state separately from input and output assets.
The predefined constructors select in-memory or file-backed components; custom
configurations can be supplied through `new_from_config()`.

### Worker

A worker implements _atomic_ functionalities that will not be broken further by the controller.
These functionalities are unrestricted and can be implemented in any language as long as they correctly implement the interface defined by the storage layer.
To facilitate the interface, workers have access to their own storage layer, see the [API](#tierkreis.worker.storage.protocol).
Typically, workers represent more expensive operations that run asynchronously.
See [Workers](#tierkreis.worker.worker.Worker).
Tierkreis can automatically generate type stubs for python workers.

### Executor

An executor is responsible for running worker tasks. The default runtime uses a
subprocess executor and discovers worker commands on `PATH`; the in-memory
runtime can execute built-in tasks only. The Rust runtime also contains a Nexus
executor, configurable through `new_from_config()`.

```{warning}
The executor reference describes the legacy Python controller. HPC schedulers,
per-task routing, and combined executors are not exposed by the Rust runtime
binding yet.
```
