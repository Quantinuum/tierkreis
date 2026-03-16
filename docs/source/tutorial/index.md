# Beginners' Tutorial

This tutorial will quickly show you the basics of using Tierkreis to
build and run a simple workflow.  If you follow the whole sequence of
lessons you will have learned enough to get started on your own
projects straight away.

Tierkreis can do many things that are not covered in this short
tutorial.  For more in depth information, and more advanced topics,
please refer to the User Guide or the API Reference.

```{toctree}
:hidden:
:maxdepth: 1
../examples/first_graph.ipynb
../examples/pytket_graph.ipynb
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
uv add tierkreis-visualization
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

### Setting up prepackaged workers

Throughout the tutorial we're going to use some prepackade worker and define your own.
To make the examples run locally you need to install the workers first:
```uv
uv add tkr-aer-worker-impl tkr-pytket-worker-impl tkr-quantinuum-worker-impl tkr-qulacs-worker-impl 
```

**TODO: make sure these packages exist.** 

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

This seems like a lot, but the default project contains a lot of
things we won't need in this tutorial.  There are really two
directories of note here: 
* `graphs` contains the graphs that define your workflow.  Making
  graphs is the main thing you will do when using Tierkreis.  We'll
  tackle this topic in the [next lesson](../examples/first_graph.ipynb).  
* `workers` is where we add custom functionality to the graph.  You
  can think of a worker as a library that is available in your
  workflow.  Code for your own workers will live here, as we describe
  in [lesson 2](../examples/pytket_graph.ipynb) and the [worker advanced guide](../worker/index.md).
 
The summary above is fairly simplified; for a more information you may
want to look at [core concepts](./core_concepts.md).
