---
file_format: mystnb
kernelspec:
  name: python3
---

# Workers

A worker implements _atomic_ functionalities that will not be broken further by the runtime.
Tierkreis workers come in three flavors:

- Builtin workers, provided by Tierkreis itself
- User defined workers
  - by using the `@worker.task()` decorator in Python (called native)
  - by wrapping arbitrary binaries  (called external)
- Prepackaged workers from the Tierkreis developers

```{important}
External workers are a way to interface non-python programs with Tierkreis.
As long as there is a runnable binary, you can provide a thin wrapper which allows Tierkreis to run this program.
```

```{toctree}
:hidden:
:maxdepth: 1
complex_types.md
external_workers.md
../examples/multiple_outputs.ipynb
native_workers/aer_worker.md
native_workers/guppy_worker.md
native_workers/ibmq_worker.md
native_workers/nexus_worker.md
native_workers/pytket_worker.md
native_workers/quantinuum_worker.md
native_workers/qulacs_worker.md
```


## The Worker Protocol

Every worker exposes named functions (tasks) with declared input and output
ports. The port names and types form the worker's interface (`api.py`), which workflows
use to connect tasks and validate values. The Tierkreis runtime manages the input and
output values through its storage layer and checkpoints them; other side effects, such as writing
additional files, are not tracked.

Workers are typically called as a binary with a single argument.
The call itself is managed by the assigned executor.
For example invoking a native worker through the subprocess executor will call the following in a shell
```
tkr-<worker-name>-impl call_args
```
Other executors can modify how this worker is invoked, e.g. wrapping the call in a SLURM batch script.
The `call_args` is the only argument; it is the path to a JSON [`WorkerCallArgs`](#tierkreis.data.location.WorkerCallArgs) document. 
It identifies the function to run and maps input and output port names to file paths. 
The worker must read each input from its path and write each result to the corresponding output path. 
It should signal successful completion using a 0 exit; on failure, it should exit with a non-zero status.
```{Important}
Deprecation notice: Some of the fields are no longer supported by the runtime:
`log_path`, `error_path`, `done_path`
```
See [External Workers](external_workers.md) for details and an example.


## Generating workers from the cli

The cli supports you in setting up new workers.
By default, we assume workers are stored in a directory `<project_root>/tkr/workers/`, you can change this using a flag in the cli.

You can generate a new worker by running:

```bash
tkr init worker --worker-name <worker-name>
```

This will generate a new worker directory and the associated files.
In the worker directory this will be (auxiallary files omitted):

```
<worker_name>/
├── api/
│    ├── api.py
│    └── pyproject.toml
├── tkr_example_worker_impl/
│   ├── __init__.py
│   ├── impl.py (task definitions)
│   └── main.py
├── README.md
└── pyproject.toml
```

Each worker is an independent project, with it's own dependencies consisting of **two** packages.

- `tkr-worker-name` contains the api definitions which you can as tasks in graph.
- `tkr-worker-name-impl` the implementation which is invoked at runtime.

The `impl.py` file is the location where you defined your worker task.
Here you can decorate a python function with `@worker.task()` to declare a worker task.
At runtime, `main.py` will be called by an executor (more on [executors](../executors/index.md)) and search for the correct task in it.
See [Running Prepackaged Workers](#running-workers) for details on executing workers.

When writing a workflow you don't need to call this function directly.
Instead you need to provide the so-called function **stubs** to the task definition, which are available in `api.py`
You can generate the stubs from the cli:

```bash
tkr init stubs
```

or running

```bash
uv run <worker_dir>/tkr_<worker_name>_impl/main.py --stubs-path <path to stubs>.py
```

This allows you to include a workers api for typechecking purposes without the need to building them and therefore making your development environment less polluted.
You can then import them using python:

```python
from worker_name import worker_function
```

For example we can run the following to generate the api for the `hello_world_worker`

```{code-cell}
!cd ../examples/example_workers/hello_world_worker && uv run tkr_hello_world_worker_impl/main.py --stubs-path ../../../worker/hello_stubs.py > /dev/null 2>&1
```

### Using workers in multiple projects

You need to write workers only once.
Since each worker declares its api as a package you can use them over multiple projects.
For this uv allows [path dependencies](https://docs.astral.sh/uv/concepts/projects/dependencies/#path) to specify the path to a different project.
In this case you would add `uv add <path>/<to>/workers/worker_name/api` which allows you to use the tasks as before.
When running you need to specify the correct registry for the executor or add the implementation as dependency
`uv add <path>/<to>/workers/worker_name`

As alternative, you are free to publish the worker packages on pypi and add them as a prepackaged worker.

## Worker code

Writing workers requires adding the `task` decorator.
Here we will provide a simple example, the full code is in [](../examples/first_graph.ipynb)
Our worker will consist of a single function returning a string.
We first instantiate a `Worker` class, which is constructed using the name of the worker.
The name of the worker tells the executor which worker a task comes from.
Therefore all of the different workers used in a single graph should have distinct names.

```{code} ipython3
from tierkreis.worker import Worker

worker = Worker("hello_world_worker")
```

The `Worker.task` method is used as a decorator to add functions to the worker.

```{code} ipython3
@worker.task()
def greet(greeting: str, subject: str) -> str: ...
```

```{note}
There are restrictions on the types of functions that `Worker.task` will accept.
If the function arguments and return types correspond to JSON types then the function will be accepted.
In addition there are a few ways of using more complex data structures,
but for now we keep things simple.
For more information please see [Complex types in Tierkreis Python workers](complex_types.md).
```

## Logging and Errors

Stderr is automatically forwarded to the runtime in case of an error.
Other outputs are discarded. 

### OpenTelemetry tracing

When runtime tracing is enabled, executors propagate
the current OpenTelemetry context to workers through environment variables.
An external worker can extract the W3C trace context from `TRACEPARENT` and,
when present, `TRACESTATE`, then use it as the parent for its own spans.
These environment names are uppercase; a standard W3C propagator may need
them mapped to the lowercase `traceparent` and `tracestate` carrier keys.

Configure an OpenTelemetry SDK and exporter in the worker to send its spans
to a collector, and flush them before the worker exits. The runtime can be
configured with `OTEL_EXPORTER_OTLP_ENDPOINT`; make the collector endpoint
available to the worker as well, including on HPC compute nodes.

## Using worker tasks

We can import tasks (here the `greet`) function from the api file and use it in `Graph.task`.

```{code-cell} ipython3
from tierkreis.builder import Graph
from tierkreis.models import TKR
from hello_stubs import greet

g = Graph(TKR[str], TKR[str])
output = g.task(greet(greeting=g.const("hello "), subject=g.inputs))
g.finish_with_outputs(output)
```

## Running Workers

Workers run through a configured runtime executor. The default subprocess
executor discovers installed worker entry points named
`tkr-<worker-name>-worker`.

```{important}
If you used the cli to generate the worker layout described above, both cases apply to your worker.
This is due to worker being also added as a package to the root project.
```

The project template adds a generated worker package to the root project, so
`uv run` exposes its entry point without a registry path. Verify it with:

```bash
uv run which tkr-worker-name-worker
```

### Running prepackaged workers

Prepackaged workers are locally installed using `uv`.
You can verify the worker is installed as package by running the `uv tree` command to list all project dependencies.

```{important}
To run the worker the package `tkr-worker-name` alone is not sufficient!
You will need the `tkr-worker-name-impl` package.
```

The implementation package declares the worker entry point. The default
runtime locates it on `PATH` and invokes it with the generated task definition:

```bash
tkr-worker-name-worker ...
```

For deployments that need an alternate command, configure `worker_commands`
on the subprocess or an HPC executor. See
[Runtime storage and executors](../executors/index.md).

## Prepackaged workers

Prepackaged workers are installed from [pypi](https://pypi.org).
There are two packages for each

```{important}
Installing `uv add tkr-<worker>` will only provide you with the stubs, not the implementation.
To run the worker inside a graph you will need `uv add tkr-<worker>-impl` too.
```

The following outlines the functionality of the prepackaged workers by the tierkreis maintainers.
Currently the following workers are provided as separate packages on pypi:

| Worker | Functionality | Documentation |
|---|---|---|
| Guppy | Compile and run quantum programs written in Guppy. | [Guppy worker](native_workers/guppy_worker.md) |
| IBMQ | Compile and run quantum circuits locally with IBMQ. | [IBMQ worker](native_workers/ibmq_worker.md) |
| Qiskit Aer | Compile and run quantum circuits locally with Qiskit Aer. | [Qiskit Aer worker](native_workers/aer_worker.md) |
| Pytket | Optimize circuits with the pytket compiler suite. The worker includes a subset of common operations; for a custom compilation pipeline, build your own worker. See this [example](../examples/hamiltonian.ipynb). | [Pytket worker](native_workers/pytket_worker.md) |
| Quantinuum Nexus | Interface to the Quantinuum Nexus platform. | [Nexus worker](native_workers/nexus_worker.md) |
| Quantinuum Backend | Compile and run quantum circuits locally with Quantinuum backends. | [Quantinuum worker](native_workers/quantinuum_worker.md) |
| Qulacs | Compile and run quantum circuits locally with Qulacs. | [Qulacs worker](native_workers/qulacs_worker.md) |

