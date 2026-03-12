# Beginners' Tutorial

This tutorial will quickly show you the basics of using Tierkreis to
build and run a simple workflow.  If you follow the whole sequence of
lessons you will have learned enough to get started on your own
projects straight away.

Tierkreis can do many things that are not covered in this short
tutorial.  For more in depth information, and more advanced topics,
please refer to the User Guide or the API Reference.

**TODO add links**

```{toctree}
:maxdepth: 1
../examples/first_graph.ipynb
../examples/hello_world_graph.ipynb
../examples/storage_and_executors.ipynb
../examples/parallelism.ipynb
../examples/hamiltonian.ipynb
whatnext.md
```

## Before you begin

Tierkreis is based on Python, and we strongly recommend using the 
[uv package manager](https://docs.astral.sh/uv/).  
You can use Tierkreis without `uv`, it's just more complex and
difficult.  For this tutorial we assume you have it available.

Let's set up a new project and install Tierkreis.
```bash
uv init
uv add tierkreis
```
The Tierkreis Visualizer is a separate package, so we'll install that too.
```bash
uv add tierkreis-visualizer
```

The Tierkreis package includes CLI tool called `tkr`.  The best way to
start a new Tierkreis development is using this tool to create a
suitable project directory.
```bash
uv run tkr init project
```

This will set up the basic project, described below.  From here you
can run an example graph as test, using the CLI:
```bash
uv run tkr run -o
> value: 1
```


**TODO: describe how to install the necessary workers and dependencies
for running the later lessons.** 

## The Default Project
The `tkr init project` command creates a directory structure like this:
```
project_root/
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
├── .gitignore
├── .python-version
├── main.py
├── pyproject.toml
├── README.md
├── uv.lock
└── workflow_inputs.json
```

From here you can run an example workflow by running

```bash
uv run tkr/graphs/main.py
> Value is: 1
```

You can also run this through the cli:
```bash
uv run tkr run -o
> value: 1
```
This will use the following default locations:
1. The workflow definition is used from `tkr/graphs/main.py:workflow`
2. The inputs are taken from `workflow_inputs.json`
3. `-o` enables the printing of outputs

### How to use the new project

The project is set up in a way to be convenient for developers already experienced with Tierkreis.
Some of the concepts we will discuss in the [core concepts](./core_concepts.md) and implement in the upcoming pages.

The repository is structure is intended to separate _graphs_, _workers_ and library code.
Graph definitions, like a `hello_world_graph` you will write in this tutorial should go into the `graphs` directory.
The main file contains an example graph, and has set up the storage and executors similar to the ones above.

`Workers` are a way to add custom functionality, which will be executed as Tierkreis tasks.
If you're not familiar with `workers` yet, we will explain them as part of this tutorial.
It is very simple to wrap existing python code to make it available in Tierkreis.
Each worker is a separate entity, your new project will contain one `example_worker`.
If you want to include more they should have a similar structure.


## Advanced Topics

Once you have finished the tutorial you can start writing your own workflows.
If you want to learn more details to fully leverage the power of Tierkreis,
the advanced  user guide available [here](../tutorial_advanced/index.md).
It includes further tutorials, and detailed descsriptions on how to write graphs, workers and executors.

### Building the examples from source

In the given examples you will be developing code involving Tierkreis workers.
Whenever you see an import containing `*_worker` this means one of the workers will be invoked (except for the `builtin`s).
```{important}
To run the examples you will need to have the worker code available too.
The simplest way to set this up is to clone the entire repository before running any of the examples.
```

```bash
git clone https://github.com/Quantinuum/tierkreis.git
```

To set up the environment we use uv:

```bash
cd tierkreis && uv sync --all-extras
```

Note that some third-party packages e.g. qulacs, automatically included via `uv sync --all-extras`, have other dependencies that you'll need to have installed on your system first: CMake and Boost (>=1.71).
When running the notebooks select the kernel corresponding to the uv environment.
