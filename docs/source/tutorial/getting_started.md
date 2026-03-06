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
├── uv.lock
└── workflow_inputs.json
```

The repository is structure is intended to separate _graphs_, _workers_ and library code.

From here you can run your first graph by running

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
1. The graph definition is used from `tkr/graphs/main.py:workflow`
2. The inputs are taken from `workflow_inputs.json`
3. `-o` enables the printing of outputs


From here you can continue with the other tutorials.

Next up is [building your first graph!](./my_first_graph.md)
