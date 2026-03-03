# Workers

A worker implements _atomic_ functionalities that will not be broken further by the controller.
Tierkreis workers come in three flavors:

- Builtin workers, provided by Tierkreis itself
- User defined workers, by using the `@worker.task()` decorator or writing external ones
- Prepackaged workers from the Tierkreis developers

```{important}
External workers are a way to interface non-python programs with Tierkreis.
As long as there is a runnable binary, you can provide a thin wrapper which allows Tierkreis to run this program.
```

```{toctree}
:maxdepth: 2
complex_types.md
external_workers.md
hello_world.md
native_workers/index
```

## Generating workers from the cli

The cli supports you in setting up new workers.
By default, we assume workers are stored in a directory `<project_root>/tkr/workers/`, you can chain this using a flag in the cli.

You can generate a new worker by running:

```
tkr init worker --worker-name <worker-name>
```

This will generate a new worker directory and the associated files.
In the worker directory this will be:

```
<worker_name>/
├── api/
│    ├── api.py
│    └── pyproject.toml
├── src/
│    ├── impl/
│    │    ├── __init__.py
│    │    └── worker_impl.py
│    └── main.py
├── README.md (this file)
└── pyproject.toml
```

Each worker is an independent project, with it's own dependencies consisting of **two** packages.

- `tkr-worker-name` contains the api definitions which you can as tasks in graph.
- `tkr-worker-name-impl` the implementation which is invoked at runtime.

The `worker_impl.py` file is the location where you defined your worker task.
Here you can decorate a python function with `@worker.task()` to declare a worker task.
At runtime, `main.py` will be called by an executor (more on [executors](../executors/index.md)) and search for the correct task in it.
See [Running Prepackaged Workers](#running-workers) for details on executing workers.

The inputs and outputs are preserved with the storage layer and be tracked by Tierkreis.
Other side-effects (e.g., writing a separate file) won't be checked.

When writing a workflow you don't need to call this function directly.
Instead you need to provide the so-called function **stubs** to the task definition, which are available in `api.py`
You can generate the stubs from the cli:

```
tkr init stubs
```

or running

```
uv run <worker_dir>/main.py --stubs-path <path to stubs>.py
```

This allows you to include a workers api for typechecking purposes without the need to building them and therefore making your development environment less polluted.
You can then import them using python:

```python
from worker_name import worker_function
```

### Using workers in multiple projects

You need to write workers only once.
Since each worker declares its api as a package you can use them over multiple projects.
For this uv allows [path dependencies](https://docs.astral.sh/uv/concepts/projects/dependencies/#path) to specify the path to a different project.
In this case you would add `uv add <path>/<to>/workers/worker_name/api` which allows you to use the tasks as before.
When running you need to specify the correct registry for the executor or add the implementation as dependency
`uv add <path>/<to>/workers/worker_name`

As alternative, you are free to publish the worker packages on pypi and add them as a prepackaged worker.

## Running Workers

In general, running workers is associated with an executor.
Running workers can happen in two flavors, which have different appropriate executors.

1. Self defined workers
2. Installed workers (we refer to them as prepackaged)

```Important
If you used the cli to generate the worker layout described above, both cases apply to your worker.
This is due to worker being also added as a package to the root project.
```

### Running self defined workers

For self defined python workers (using `main.py`) we use the `UvExecutor` as follows:

```python
executor = UvExecutor(
    Path("<project_root>/tkr/workers/"), storage.logs_path
)
```

When running the graph with this executor, tierkreis will search for the directory `<project_root>/tkr/workers/worker_name/src` and inside execute the command

```bash
uv run main.py ...
```

As a fallback it will try `<project_root>/tkr/workers/worker_name/`.
When you want to reuse this worker in a different project (`project_B`) you can use the same executor but need to update the registry path to know point to the correct relative path.

### Running prepackaged workers

Prepackaged workers are locally installed using `uv`.
You can verify the worker is installed as package by running the `uv tree` command to list all project dependencies.

```Important
To run the worker the package `tkr-worker-name` alone is not sufficient!
You will need the `tkr-worker-name-impl` package.
```

The workers declare an export script running `tkr-worker-name` which is available in your environment if you installed the correct package.
Running `which tkr-worker-name` will confirm its existence.
Hence it can be treated as a shell script; therefore we use the `ShellExecutor` for it like so:

```python
executor = ShellExecutor(Path(), storage.workflow_dir)
```

This will run through a similar verification process to locate the command and then run:

```bash
tkr-worker-name ...
```

## Prepackaged workers

Prepackaged workers are installed from [pypi](https://pypi.org).
There are two packages for each

```{important}
Installing `uv add tkr-<worker>` will only provide you with the stubs, not the implementation.
To run the worker inside a graph you will need `uv add tkr-<worker>-impl` too.
```

### TKR Workers

The following outlines the functionality of the prepackaged workers by the tierkreis maintainers.
Currently the following workers are provided as separate packages on pypi:

- IBMQ
- Qiskit Aer
- Pytket
- Quantinuum Nexus
- Quantinuum Backend
- Qulacs

#### Qiskit Aer

Compile and run quantum circuits locally with Qiskit Aer.
More detailed docs [here](native_workers/aer_worker.md).

#### IBMQ

Compile and run quantum circuits locally with IBMQ.
More detailed docs [here](native_workers/ibmq_worker.md).

#### Pytket

The pytket compiler suite to optimize circuits.
The worker only contains a subset of common operations.
For a custom compilation pipeline it is advised to build your own worker.

More detailed docs [here](native_workers/pytket_worker.md).

**Installation**

```sh
pip install tkr-pytket-worker
```

will install an executable Python script `tkr_pytket_worker` into your virtual environment.

**Example**

See the example `hamiltonian_graph.py`.

#### Quantinuum Nexus

Interface to the Quantinuum Nexus platform.
More detailed docs [here](native_workers/nexus_worker.md).

#### Quantinuum Backend

Compile and run quantum circuits locally with Quantinuum backends.
More detailed docs [here](native_workers/quantinuum_worker.md).

#### Qulacs

Compile and run quantum circuits locally with Qulacs.
More detailed docs [here](native_workers/qulacs_worker.md).
