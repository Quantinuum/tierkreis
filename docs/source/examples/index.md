# Examples

This directory contains examples how to use Tierkreis to write workflows.
Before writing workflows, we will shortly recap how to write a worker.
It is intended that these notebooks are read in the following order.

```{toctree}
:maxdepth: 1
worker.ipynb
hello_world_graph.ipynb
storage_and_executors.ipynb
types_and_defaults.ipynb
multiple_outputs.ipynb
polling_and_dir.ipynb
parallelism.ipynb
errors_and_debugging.ipynb
restart.ipynb
signing_graph.ipynb
hpc.ipynb
scipy.ipynb
hamiltonian.ipynb
qsci.ipynb
```

## How to use

In the given examples you will be developing code involving Tierkreis workers.
Whenever you see an import containing `*_worker` this means one of the workers will be invoked (except for the `builtin`s).
To ensure the examples will run correctly you will need to have the worker code available too.
The simplest way to set this up is to clone the entire repository before running any of the examples.

```
git clone https://github.com/Quantinuum/tierkreis.git
```

To set up the environment we us uv:

```
uv sync --all-extras
```

When running the notebooks select the kernel corresponding to the uv environment.
