# TKR: Tierkreis CLI

`tkr` is the Rust command-line interface for creating projects and workers,
generating worker APIs, running workflows, and operating the Tierkreis runtime.

Use `tkr --help` or `tkr <command> --help` for the complete command reference.

## Create a project

```bash
tkr init project [DIRECTORY]
```

The command creates:

- `tierkreis.toml`, containing project-local runtime, storage, and logging paths;
- `tkr/graphs/main.py`, an example workflow;
- `tkr/workers/example_worker`, an example Python worker and API package;
- `workflow_inputs.json`; and
- a minimal `pyproject.toml` when the directory does not already have one.

Initialization never overwrites existing generated paths. If a `pyproject.toml`
already exists it is left unchanged, so ensure it depends on `tierkreis`, then run
`uv sync`.

The old `--default-checkpoint-directory`, `--graphs-directory`, and
`--worker-directory` project options have been removed. Runtime paths belong in
`tierkreis.toml`, and initialized projects use the standard `tkr/` layout.

## Create workers

```bash
# Discover tkr/workers from the nearest tierkreis.toml
tkr init worker my-worker

# Use an explicit worker collection
tkr init worker my-worker --directory ./custom-workers

# Create a non-Python worker schema and Python API package
tkr init worker external-worker --external
```

Worker names may contain ASCII letters, digits, hyphens, and underscores and must
start with a letter. Python modules use underscores while distribution names use
hyphens.

## Generate worker APIs

```bash
tkr generate stubs
tkr generate stubs --workers-directory ./custom-workers
tkr generate stubs --output api/api.py
```

The Rust CLI discovers and orders workers, then invokes the Python worker or the
private TypeSpec adapter needed to inspect its interface. `uv` must be installed,
and child-process failures stop generation with a non-zero exit status.

## Run a workflow

Run a serialized current or legacy workflow graph with JSON inputs:

```bash
tkr run workflow.json --inputs inputs.json --print-output
```

During the migration, Python graph factories are supported through an explicit
adapter:

```bash
tkr run --python package.workflows:main --inputs inputs.json
tkr run --python ./graphs/main.py:workflow --inputs inputs.json
```

Options include:

| Option | Description |
|---|---|
| `-i, --inputs PATH` | JSON object of workflow inputs; defaults to `workflow_inputs.json` |
| `--name NAME` | Human-readable workflow name |
| `-o, --print-output` | Print top-level outputs as formatted JSON |
| `--config PATH` | Use an explicit `tierkreis.toml` |
| `--python GRAPH` | Load a Python `module:function` or `path.py:function` |

The CLI prints generated workflow and run UUIDs. Executor, state, storage, logging,
and scheduler choices are configured in `tierkreis.toml`; legacy controller flags
such as `--uv`, `--registry-path`, iteration limits, and polling intervals are no
longer command-line options.

## Operate services

```bash
# Run the workflow runtime until interrupted
tkr exec [--config ./tierkreis.toml]

# Serve the API and Swagger UI
tkr serve [--config ./tierkreis.toml]

# Also serve an explicitly built visualization frontend
tkr serve --assets ./tierkreis_visualization/tierkreis_visualization/static/dist
```

Both commands discover `tierkreis.toml` in the current directory or an ancestor,
then fall back to `TIERKREIS_CONFIG` and the user-level configuration. The server
always exposes its API and Swagger UI; the browser frontend is enabled when
`--assets` points to a built frontend. Hot reload remains a separate visualization
development workflow.
