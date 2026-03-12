# TKR: Tierkreis CLI Tool

Tierkreis includes a comprehensive command-line interface to manage workflows, workers, and other aspects of your project. 
This document provides a complete reference for all available commands and options.

## Overview

The main `tkr` command has three primary subcommands:

- **`init`** — Initialize and manage Tierkreis project resources
- **`run`** — Execute workflow graphs
- **`vis`** — Visualize and inspect graphs

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
uv run tkr init stubs --worker-directory ./my-workers
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

**Graph Specification:**

You can specify the graph in two ways:

1. **Module path (default):** `module.path:function_name`
   - Example: `examples.hello_world:hello_graph`
   - Example: `./my_graphs/workflow.py:my_workflow`

2. **JSON file:** Use `-f/--from-file` to load a serialized graph

**Input Specification:**

You can provide inputs in two ways:

1. **JSON file (default):** A single `.json` file containing input values
   - Example: `workflow_inputs.json`

2. **Binary files:** Key-value pairs where `key` is the input port name and `path` is a file containing binary data
   - Example: `port1:input1 port2:input2`

Where input1 might look like

**Options:**

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `-g, --graph-location` | STRING | `./tkr/graphs/main.py:workflow` | Load graph from Python module or file |
| `-f, --from-file` | PATH | `workflow_inputs.json` | Load a serialized graph from a `.json` file |
| `-i, --input-files` | LIST | - | Input sources: `key:path` pairs |
| `-o, --print-output` | FLAG | False | Print the outputs of the top-level graph node |
| `--name` | STRING | — | Workflow name (for identification in logs) |
| `--run-id` | INT | — | Numeric run ID for tracking |
| `-n, --n-iterations` | INT | — | Maximum number of iterations for the workflow |
| `-p, --polling-interval-seconds` | FLOAT | — | Controller polling interval (tick rate) |
| `-r, --do-clean-restart` | FLAG | False | Clear all graph files before running |
| `--uv` | FLAG | False | Use `uv` as the executor |
| `--registry-path` | PATH | None | Location of executable tasks/binaries |
| `-l, --loglevel` | {CRITICAL, FATAL, ERROR, WARN, WARNING, INFO, DEBUG, NOTSET} | WARNING | Set logging level |
| `-v, --verbose` | FLAG | False | Enable verbose output |

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

# Provide custom inputs as binary files
uv run tkr run -i value:input.bin count:count.bin

# Run with custom configuration
uv run tkr run \
  -g examples.quantum:circuit \
  -i inputs.json \
  --name "quantum-run-001" \
  -o -v

# Clean restart and run with custom tick rate
uv run tkr run -r -p 0.5 --uv

# Set logging level and run ID
uv run tkr run \
  -l DEBUG \
  --run-id 42 \
  --n-iterations 100

# Use custom registry path
uv run tkr run --registry-path /path/to/tasks
```

**What happens:**
1. The graph is loaded from the specified location
2. Inputs are loaded from the specified files
3. The workflow is executed by the Tierkreis controller
4. If `-o` is specified, outputs are printed to console
5. Checkpoints are saved to `TKR_DIR` (default: `~/.tierkreis/checkpoints`)

---

## tkr vis

Visualize and interact with Tierkreis graphs.

**Usage:**
```bash
uv run tkr vis [OPTIONS]
```

**Description:**
Launches the Tierkreis visualizer, which provides an interactive web interface for viewing and debugging graphs.
 This is useful for understanding workflow structure, inspecting node connections, and debugging execution flow.

**Modes:**

The visualization tool has t main modes:

1. **Default Mode:** Start the visualization server showing all workflows run in the past.
w. **Graph Mode:** Visualize a specific graph object that has was not run.

**Options:**

| Option | Type | Description |
|--------|------|-------------|
| `--dev` | FLAG | Run in development mode with hot reload on code changes |
| `--graph` | STRING | Visualize a specific graph: `<path_to_python_file>:<variable_name>` similar to `tkr run --g` |
| `-h, --help` | FLAG | Show help message |

**Examples:**

```bash
# Start the visualization server (interactive web UI)
uv run tkr vis

# Start in development mode (with hot reload)
uv run tkr vis --dev

# Visualize a specific graph
uv run tkr vis --graph ./my_graphs/workflow.py:main_workflow

# Visualize a graph from a module
uv run tkr vis --graph my_module.graphs:visualization_graph
```

**Access the UI:**

After running `uv run tkr vis`, the server starts at `http://localhost:8000` .
Open this URL in your web browser to access the visualization.
More info on the [visualizer](visualization.md) page.
