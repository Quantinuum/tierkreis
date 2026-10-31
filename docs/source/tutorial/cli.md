# TKR: Tierkreis CLI Tool

Tierkreis includes a comprehensive command-line interface to manage workflows, workers, and other aspects of your project. 
This document provides a complete reference for all available commands and options.

## Overview

The main `tkr` command has three primary subcommands:

- **`init`** — Initialize and manage Tierkreis project resources
- **`run`** — Execute workflow graphs
- **`serve`** — View persisted workflows without executing them

Use `--help` on any of the commands for a quick overview, or consult the sections below for detailed information about each command.

---

## tkr init

Initialize and manage Tierkreis project resources. The `init` command has three subcommands: `project`, `worker`, and `stubs`.

### tkr init project

Sets up a new Tierkreis project and manages project-wide options. Make sure to set up a Python project first using `uv init`.

**Usage:**
```bash
uv run tkr init project [OPTIONS]
```

**Description:**
This command initializes the directory structure for a Tierkreis project, including directories for graphs and workers. You can customize the default locations for these resources.

**Options:**

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `--project-directory` | PATH | `.` (current directory) | Root directory for the project |
| `--graphs-directory` | PATH | `./tkr/graphs` | Location where workflow graphs are stored |
| `--worker-directory` | PATH | `./tkr/workers` | Location where worker implementations are stored |
| `--default-checkpoint-directory` | PATH | `~/.tierkreis/checkpoints` | Where workflow checkpoints are saved. Also sets the `TKR_DIR` environment variable for the current shell |

**Examples:**

```bash
# Initialize with default directories
uv run tkr init project

# Use custom directories
uv run tkr init project \
  --project-directory ./my-project \
  --graphs-directory ./my-graphs \
  --worker-directory ./my-workers

# Set a custom checkpoint directory
uv run tkr init project \
  --default-checkpoint-directory /data/checkpoints

# Persist checkpoint directory (add to shell config file, e.g., ~/.bashrc or ~/.zshrc)
export TKR_DIR=/data/checkpoints
```

### tkr init worker

Generate a new worker template. 
Workers are independent units that provide tasks to graphs.

**Usage:**
```bash
uv run tkr init worker -n WORKER_NAME [OPTIONS]
```

**Description:**
Creates a new worker with boilerplate code.
By default, it generates a Python worker with a fixed project structure. 
Use `--external` to generate an example typespec file instead for non-Python workers.

**Options:**

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `-n, --worker-name` | STRING | — | Name of the worker to create |
| `--worker-directory` | PATH | `./tkr/workers` | Override the default worker directory |
| `--external` | FLAG | False | Generate an external (non-Python) worker with IDL file |

**Examples:**

```bash
# Create a Python worker
uv run tkr init worker -n my-worker

# Create with custom directory
uv run tkr init worker -n alternative-worker --worker-directory ./custom_workers

# Create an external (non-Python) worker
uv run tkr init worker -n external-worker --external
```

### tkr init stubs

Generate the API files for workers using `uv`.
The APIs provide the task names during graph construction 

**Usage:**
```bash
uv run tkr init stubs [OPTIONS]
```

**Description:**
Scans the worker directory and generates python stub files (`api.py`) for type hints and IDE autocomplete.
This is **required** after creating or modifying worker interfaces.

**Options:**

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `--worker-directory` | PATH | `./tkr/workers` | Directory to search for workers |
| `--api-file-name` | PATH | `./api/api.py` | Output location for generated API stubs (relative to worker directory) |

```{warning}
Changing the api file name might break your worker.
Use only when appropriate.
```

**Examples:**

```bash
# Generate stubs with defaults
uv run tkr init stubs


# Generate stubs for workers in custom directory
uv run tkr init stubs --worker-directory ./custom_workers
```
---

## tkr run

Execute a workflow graph with specified inputs and configuration.

**Usage:**
```bash
uv run tkr run [OPTIONS]
```

**Description:**
Runs a Tierkreis workflow graph. By default, it loads the graph from `./tkr/graphs/main.py:workflow` and reads inputs from `workflow_inputs.json`.
You can specify a different graph location or load from a JSON file, and provide inputs either as a JSON file or as binary files with key-value pairs.

Use `--profile NAME` to select a resource profile for named execution contexts.
Without this option, the runtime's default profile selection is preserved.
Use `--timeout SECONDS` to change the completion wait limit (default: 30 seconds);
the value is an integer number of seconds. A wait timeout does not cancel the run.
Pending telemetry is flushed before the CLI exits, including after failures.

```bash
uv run tkr run -g my_project.graphs.he:workflow -i he_input.json \
  --config runtime.toml --profile local --timeout 86400 -o
```

**Graph Specification:**

You can specify the graph in two ways:

1. **Module path (default):** `module.path:function_name`
   - Example: `examples.hello_world:hello_graph`
   - Example: `./my_graphs/workflow.py:my_workflow`

2. **Serialized graph file:** Use `-f/--from-file` to load a graph

**Input Specification:**

You can provide inputs in two ways:

1. **JSON file (default):** A single `.json` file containing input values
  - Example: `workflow_inputs.json` e.g.
  ```
  { "value": 1 }
  ```

2. **Binary files:** Key-value pairs where `key` is the input port name and `path` is a file containing binary data
  - Example: `port1:input1 port2:input2` e.g. `input1` contains
  ```
  b"1"
  ```

**Options:**

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `-g, --graph-location` | STRING | `./tkr/graphs/main.py:workflow` | Load graph from Python module or file |
| `-f, --from-file` | PATH | — | Load a serialized graph from a file |
| `-i, --input-files` | LIST | `workflow_inputs.json` | One JSON file, `key:path` pairs, or `None` |
| `-o, --print-output` | FLAG | False | Print the outputs of the top-level graph node |
| `--name` | STRING | — | Workflow name (for identification in logs) |
| `--config` | PATH | discovered default | Runtime TOML configuration |
| `-l, --loglevel` | {CRITICAL, FATAL, ERROR, WARN, WARNING, INFO, DEBUG, NOTSET} | WARNING | Set logging level |
| `-v, --verbose` | FLAG | False | Enable verbose output |


Installed subprocess workers are discovered as `tkr-<worker-name>-worker`.
Alternative commands, executors, asset stores, and resource profiles belong in
the runtime configuration; see [Executors](../executors/index.md).

**Examples:**

```bash
# Run with all defaults
uv run tkr run
# Loads from ./tkr/graphs/main.py:workflow and workflow_inputs.json

# Print outputs to console
uv run tkr run -o
# Same as above but displays the workflow outputs

# Run a specific graph
uv run tkr run -g my_module.workflows:main_workflow

# Load graph from file
uv run tkr run -f my_graph.json

# Provide custom inputs as files containing binary values e.g. b"5"
uv run tkr run -i value:input count:count

# Run with custom configuration
uv run tkr run \
  -g examples.quantum:circuit \
  -i inputs.json \
  --config ./tierkreis.toml \
  --name "quantum-run-001" \
  -o -v
```

**What happens:**
1. The graph is loaded from the specified location
2. Inputs are loaded from the specified files
3. The graph is saved and executed by the Rust runtime
4. If `-o` is specified, outputs are printed to console
5. State and assets are stored according to the runtime configuration

---

## tkr serve

View persisted Tierkreis workflows without starting or resuming execution.

**Usage:**
```bash
uv run tkr serve [OPTIONS]
```

**Description:**
Launches the Rust visualization server using the discovered runtime configuration.
The server opens the configured persistent SQLite database and asset storage;
it does not initialize executors or run the orchestrator. In-memory runtime
state is rejected because it cannot contain persisted runs. An explicit
`runtime_state.url` takes precedence over `DATABASE_URL`, matching `tkr run`.
Relative configuration paths resolve from the current working directory.

**Options:**

| Option | Type | Description |
|--------|------|-------------|
| `--host` | IP ADDRESS | Bind address; defaults to `127.0.0.1` |
| `--port` | INTEGER | HTTP port; defaults to `3000` |
| `--config` | PATH | Runtime TOML file; defaults to standard configuration discovery |
| `-h, --help` | FLAG | Show help message |

**Examples:**

```bash
# Start the visualization server (interactive web UI)
uv run tkr serve

# Use an explicit configuration and port
uv run tkr serve --config runtime.toml --port 3001

# Bind on a coordinator's network interface (secure access externally)
uv run tkr serve --host 0.0.0.0
```

**Access the UI:**

After running `uv run tkr serve`, the server starts at `http://localhost:3000`.
Open this URL in your web browser to access the visualization.
The viewer has no authentication; use loopback binding or an authenticated tunnel.
More info on the [visualizer](visualization.md) page.
