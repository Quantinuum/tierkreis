# Beginners Tutorial

This tutorial will guide you through writing your own workflows with Tierkreis.
First it will cover the core concepts of workflow systems and how they are implemented in Tierkreis.
Then we will start with simple graph examples, providing custom tasks building up to more complex examples.
It is intended that these notebooks are read in the following order.

```{toctree}
:maxdepth: 1
core_concepts.md
../examples/first_graph.ipynb
visualization.md
../examples/hello_world_graph.ipynb
../examples/storage_and_executors.ipynb
../examples/parallelism.ipynb
../examples/hamiltonian.ipynb
```

## How to use

In the given examples you will be developing code involving Tierkreis workers.
Whenever you see an import containing `*_worker` this means one of the workers will be invoked (except for the `builtin`s).
To ensure the examples will run correctly you will need to have the worker code available too.
The simplest way to set this up is to clone the entire repository before running any of the examples.

```bash
git clone https://github.com/Quantinuum/tierkreis.git
```

To set up the environment we use uv:

```bash
(cd tierkreis && uv sync --all-extras)
```

Note that some third-party packages e.g. qulacs, automatically included via `uv sync --all-extras`, have other dependencies that you'll need to have installed on your system first: CMake and Boost (>=1.71).

When running the notebooks select the kernel corresponding to the uv environment.

### Creating your own Tierkreis project
If you instead want to immediately develop your own project, you can use  the `tkr` cli to set up your own project

After running

```bash
uv init
uv add tierkreis
```

(and optionally `uv add tierkreis-visualization` ?)

Your local environment will now have access to the `tkr` cli.
Running

```bash
uv run tkr init project
```
(you can ignore the warnings about tierkreis-visualization not being installed...)
and you should now have the following project structure in your current directory:

```
├── tkr/
│   ├── graphs/
│   │   └── main.py
│   └── workers/
│       └── example_worker/
│             ├── api/
│             │   ├── api.py (stubs for the worker)
│             │   ├── pyproject.toml
│             │   └── README.md
│             ├── tkr_example_worker_impl/
│             │   ├── __init__.py
│             │   ├── impl.py (task definitions)
│             │   └── main.py
│             ├── __init__.py
│             ├── pyproject.toml
│             ├── README.md
│             └── uv.lock
├── .git                } All of
├── .gitignore.         } these were
├── .python-version.    } actually
├── main.py.            } added by
├── pyproject.toml.     } `uv init`
├── README.md           } earlier
├── uv.lock           -- and this one by `uv add tierkreis`
```

From here you can run an example graph by running

```bash
uv run python -m tkr.graphs.main
> Value is: 1
```

Note alternative suggestion does not seem to work:
```bash
$ uv run tkr/graphs/main.py
Traceback (most recent call last):
  File "/Users/alanlawrence/tierkreis-trial/tkr/graphs/main.py", line 11, in <module>
    from tkr.workers.example_worker.api.api import your_worker_task
ModuleNotFoundError: No module named 'tkr'
```

You can also run this through the cli:
```bash
uv run tkr run -o
-- sadly not, resulted in usage message plus "tkr run: error: one of the arguments -f/--from-file -g/--graph-location is required"
> value: 1
```

This will use the following default locations:
1. The graph definition is used from `tkr/graphs/main.py:workflow` -- there is no such variable
2. The inputs are taken from `workflow_inputs.json` -- there is no such file
3. `-o` enables the printing of outputs

### How to use the new project

The project is set up in a way to be convenient for developers already experienced with Tierkreis.
Some of the concepts we will discuss in the [core concepts](./core_concepts.md) and implement in the upcoming pages.

The repository is structure is intended to separate _graphs_, _workers_ and library code.
Graph definitions, like a `hello_world_graph` you will write in this tutorial should go into the `graphs` directory.
The main file contains an example graph, and has set up the storage and executors similar to the ones above.

`Workers` are a way to add custom functionality, which will be executed as Tierkreis tasks.
If you're not familiar with `workers` yet, we will explain them as part of this tutorial. -- Is this upcoming, or is "this tutorial" another file somewhere else?
It is very simple to wrap existing python code to make it available in Tierkreis.
Each worker is a separate entity, your new project will contain one `example_worker`.
If you want to include more they should have a similar structure.


## Advanced Topics

Once you have finished the tutorial you can start writing your own workflows.
If you want to learn more details to fully leverage the power of Tierkreis,
the advanced user guide available [here](../tutorial_advanced/index.md).
It includes further tutorials, and detailed descriptions on how to write graphs, workers and executors.
