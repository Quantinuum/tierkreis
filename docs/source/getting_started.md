# Getting Started

## Project Setup

We recommend using [uv](https://docs.astral.sh/uv/).
You can set up a project using `uv` and the tierkreis cli called `tkr`.

```bash
uv init
uv add tierkreis
tkr init project
```

This will set up the following project structure for you:

```
project_root/
├── tkr/
│   ├── graphs/
│   │   ├── __init__.py
│   │   └── main.py
│   └── workers/
│       ├── __init__.py
│       └── example_worker/
│           ├── __init__.py
│           ├── main.py
│           ├── pyproject.toml
│           ├── README.md
│           ├── stubs.py
│           └── uv.lock
├── .gitignore
├── .python-version
├── main.py
├── pyproject.toml
├── README.md
└── uv.lock
```

The repository is structure is intended to separate _graphs_, _workers_ and library code.

From here you can run your first graph by running

```
uv run python -m tkr.graphs.main
> Value is: 1
```

From here you can continue with the other tutorials.

## Step by step tutorials

A sequence of tutorials that cover the fundamentals of writing and operating Tierkreis workflows.

[Our first graph](tutorial/builtins.md)

[Graph inputs and outputs](tutorial/inputs.md)

[Nested graphs using Eval](tutorial/eval.md)

[Iteration using Loop](tutorial/loop.md)

[Parallel computation using Map](tutorial/map.md)

## Worker examples

Tutorials on writing workers that provide additional tasks.
For worker libraries see [this document](workers.md)

### Tierkreis Python library

[Hello world worker](worker/hello_world.md)

[Complex types in Tierkreis Python workers](worker/complex_types.md)

### Other Workers

[External workers with an IDL](worker/external_workers.md)

## Executors

[Overview](executors/overview.md)

[Shell Executors](executors/shell.md)

[HPC Executors](executors/hpc.md)
