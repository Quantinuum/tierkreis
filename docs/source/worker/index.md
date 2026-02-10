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
You can declare a worker by running:

```
tkr init worker --worker-name <worker-name>
```

This will generate a new worker directory and the associated files.
Each worker is an independent project, with it's own dependencies.
The `main.py` file is the entrypoint for the worker.
Here you can decorate a python function with `@worker.task()` to declare a worker task.
At runtime, `main.py` will be called by an executor (more on that later) and search for the correct task in it.
The inputs and outputs are preserved with the storage layer and be tracked by Tierkreis.
Other side-effects (e.g., writing a separate file) won't be checked.

When writing a workflow you don't need to call this function directly.
Instead you need to provide the so-called function **stubs** to the task definition, which are available as the workers api.
You can generate the stubs from the cli:

```
tkr init stubs
```

or running

```
uv run <worker_dir>/main.py --stubs-path <path to stubs>.py
```

This allows you to include a workers api for typechecking purposes without the need to building them and therefore making your development environment less polluted.

## Prepackaged workers

The following outlines the functionality of the prepackaged workers.
Currently the following workers are provided as separate packages on pypi:

- IBMQ
- Qiskit Aer
- Pytket
- Quantinuum Nexus
- Quantinuum Backend

### Qiskit Aer

Compile and run quantum circuits locally with Qiskit Aer.
More detailed docs [here](worker/native_workers/aer_worker.md).

### IBMQ

Compile and run quantum circuits locally with IBMQ.
More detailed docs [here](worker/native_workers/ibmq_worker.md).

### Pytket

The pytket compiler suite to optimize circuits.
The worker only contains a subset of common operations.
For a custom compilation pipeline it is advised to build your own worker.

More detailed docs [here](worker/native_workers/pytket_worker.md).

**Installation**

```sh
pip install tkr-pytket-worker
```

will install an executable Python script `tkr_pytket_worker` into your virtual environment.

**Example**

See the example `hamiltonian_graph.py`.

### Quantinuum Nexus

Interface to the Quantinuum Nexus platform.
More detailed docs [here](worker/native_workers/nexus_worker.md).

### Quantinuum Backend

Compile and run quantum circuits locally with Quantinuum backends.
More detailed docs [here](worker/native_workers/quantinuum_worker.md).

### Qulacs

Compile and run quantum circuits locally with Qulacs.
More detailed docs [here](worker/native_workers/qulacs_worker.md).
