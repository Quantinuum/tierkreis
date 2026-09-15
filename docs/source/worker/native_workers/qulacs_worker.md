# Qulacs worker

A Tierkreis worker that compiles and runs circuits with Qulacs.

The Qulacs worker largely wraps the functionality from [pytket-qulacs](https://github.com/Quantinuum/pytket-qulacs/).
In addition to the elementary tasks exposed, there are also prepackaged graphs to make using the worker more convenient.

## Installation

```sh
pip install tkr-qulacs-worker
```

will install the `qulacs_worker` api into your virtual environment.

```sh
pip install tkr-qulacs-worker-impl
```

will install the `tkr-qulacs-worker` script into your virtual environment.

## Elementary tasks

The Qulacs worker exposes the following elementary tasks to the user.

- `get_compiled_circuit`. A wrapper around `QulacsBackend.get_compiled_circuit`, which is intended to be parallelised using a Tierkreis `map`.
- `run_circuit`. A wrapper around `QulacsBackend.run_circuit`, which is intended to be parallelised using a Tierkreis `map`.
- `run_circuits`. A runs multiple circuits according to the logic defined in the `process_circuits` method in [pytket-qulacs](https://github.com/Quantinuum/pytket-qulacs/blob/main/pytket/extensions/qulacs/backends/qulacs_backend.py#L186).

The full api is available in the {py:mod}`API Docs <qulacs_worker>`.

## Prepackaged graphs

The Tierkreis Python package provides a few prepackaged graphs to make it easier to compile and run circuits with Qulacs.

`tierkreis.graphs.simulate.compile_simulate.compile_simulate` is intended for the common use case of compiling a list of circuits in parallel and then running them in parallel.
It can be included within a custom graph using `Graph.eval` or run as a standalone graph.

An example use is in `docs/source/examples/parallelism.ipynb` in the [Tierkreis repo](https://github.com/Quantinuum/tierkreis), which looks like:

```python
from tierkreis import new_default

simulator_name = "qulacs"
circuits = ...your circuits here...

g = compile_simulate()
runtime = await new_default()
inputs = {
    "circuits": [circuit.to_dict() for circuit in circuits],
    "n_shots": [30] * len(circuits),
    "config": config,
    "compilation_optimisation_level": 2,
}
with runtime:
    workflow_id = await runtime.save_workflow("qulacs simulation", g)
    run_id = await runtime.start_new_run(workflow_id, inputs)
    await runtime.wait_for(run_id, 0)
    res = await runtime.get_outputs(run_id, 0)
print(len(res))

```

The subgraph `tierkreis.graphs.simulate.compile_simulate.compile_simulate_single` can also be used if the user wants to do additional customisations.
