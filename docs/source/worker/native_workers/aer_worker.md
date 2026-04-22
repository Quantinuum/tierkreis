# Qiskit Aer worker

A Tierkreis worker that compiles and runs circuits with Qiskit Aer.

The Aer worker largely wraps the functionality from [pytket-qiskit](https://github.com/Quantinuum/pytket-qiskit/).
In addition to the elementary tasks exposed, there are also prepackaged graphs to make using the worker more convenient.

## Installation

```sh
pip install tkr-aer-worker
```

will install the `aer_worker` api into your virtual environment.

```sh
pip install tkr-aer-worker-impl
```

will install the `tkr-aer-worker` script into your virtual environment.

## Elementary tasks

The Aer worker exposes the following elementary tasks to the user.

- `get_compiled_circuit`. A wrapper around `AerBackend.get_compiled_circuit`, which is intended to be parallelised using a Tierkreis `map`.
- `run_circuit`. A wrapper around `AerBackend.run_circuit`, which is intended to be parallelised using a Tierkreis `map`.
- `run_circuits`. A wrapper around `AerBackend.run_circuits`, which performs multiple simulations according to the logic in the `process_circuits` method in [pytket-qiskit](https://github.com/Quantinuum/pytket-qiskit/blob/main/pytket/extensions/qiskit/backends/aer.py).
- `to_qasm3_str`. Converts a pytket `Circuit` to QASM3 using the Qiskit qasm3 module.

The full api is available in the {py:mod}`API Docs <aer_worker>`.

## Prepackaged graphs

The Tierkreis Python package provides a few prepackaged graphs to make it easier to compile and run circuits with Aer.

`tierkreis.graphs.simulate.compile_simulate.compile_simulate` is intended for the common use case of compiling a list of circuits in parallel and then running them in parallel.
It can be included within a custom graph using `Graph.eval` or run as a standalone graph.

An example use is in `docs/source/examples/parallelism.ipynb` in the [Tierkreis repo](https://github.com/Quantinuum/tierkreis), which looks like:

```python
simulator_name = "aer"
circuits = ...your circuits here...

g = compile_simulate()
storage = FileStorage(UUID(int=107), do_cleanup=True)
executor = UvExecutor(PACKAGE_PATH / ".." / "tierkreis_workers", storage.logs_path)

run_graph(
    storage,
    executor,
    g,
    {
        "circuits": circuits,
        "n_shots": [30] * len(circuits),
        "config": config,
        "compilation_optimisation_level": 2,
    },
    polling_interval_seconds=0.1,
)
res = read_outputs(g, storage)
print(len(res))

```

The subgraph `tierkreis.graphs.simulate.compile_simulate.compile_simulate_single` can also be used if the user wants to do additional customisations.
