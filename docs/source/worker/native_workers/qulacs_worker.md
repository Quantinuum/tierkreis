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
simulator_name = "qulacs"
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
