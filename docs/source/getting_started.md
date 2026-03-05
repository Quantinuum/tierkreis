# Getting Started

## Project Setup

Tierkreis works best with the [uv package manager](https://docs.astral.sh/uv/). We strongly recommend using it as your package manager for Tierkreis projects.

To get started with Tierkreis start a new `uv` project in an empty directory with:

```bash
uv init
```

Then add Tierkreis to the project and run the project setup tool.

```bash
uv add tierkreis
uv run tkr init project
```

This will set up the following project structure for you:

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
└── uv.lock
```

The repository is structure is intended to separate _graphs_, _workers_ and library code.

From here you can run your first graph by running

```bash
uv run tkr/graphs/main.py
> Value is: 1
```

You can also run this through the cli:
```bash
uv run tkr run -i tkr/graphs/inputs.json -o
> value: 1
```


From here you can continue with the other tutorials.

## Tutorials for writing workflows

A sequence of tutorials that cover the fundamentals of writing and operating Tierkreis workflows.

[Our first graph](tutorial/builtins.md)

[Graph inputs and outputs](tutorial/inputs.md)

[Nested graphs using Eval](tutorial/eval.md)

[Iteration using Loop](tutorial/loop.md)

[Parallel computation using Map](tutorial/map.md)

## Tutorials for writing workers

Tutorials on writing workers that provide additional tasks.
For a general overview look at the [worker documentation](worker/index.md)
For worker libraries see [this document](worker/native_workers/index.md)

### Tierkreis Python library

[Hello world worker](worker/hello_world.md)

[Complex types in Tierkreis Python workers](worker/complex_types.md)

### Other Workers

[External workers with an IDL](worker/external_workers.md)

## Executors

[Overview](executors/index.md)

[Shell Executors](executors/shell.md)

[HPC Executors](executors/hpc.md)
