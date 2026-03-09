# Beginners Tutorial

<!--TODO This beginner tutorial contains examples how to use Tierkreis to write workflows.
Before writing workflows, we will shortly recap how to write a worker.
It is intended that these notebooks are read in the following order. -->

```{toctree}
:maxdepth: 1
core_concepts.md
getting_started.md
my_first_graph.md
visualization.md
../examples/worker.ipynb
../examples/hello_world_graph.ipynb
../examples/storage_and_executors.ipynb
../examples/types_and_defaults.ipynb
../examples/polling_and_dir.ipynb
../examples/parallelism.ipynb
../examples/errors_and_debugging.ipynb
logging_and_errors.md
../examples/restart.ipynb
../examples/signing_graph.ipynb
../examples/hpc.ipynb
../examples/scipy.ipynb
../examples/hamiltonian.ipynb
../examples/qsci.ipynb
further_topics.md
```

## How to use

In the given examples you will be developing code involving Tierkreis workers.
Whenever you see an import containing `*_worker` this means one of the workers will be invoked (except for the `builtin`s).
To ensure the examples will run correctly you will need to have the worker code available too.
The simplest way to set this up is to clone the entire repository before running any of the examples.

```
git clone https://github.com/Quantinuum/tierkreis.git
```

To set up the environment we use uv:

```
uv sync --all-extras
```

When running the notebooks select the kernel corresponding to the uv environment.
