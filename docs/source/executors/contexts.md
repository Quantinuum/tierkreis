# Execution context and profiles

An **execution context** is a portable name attached to a task or nested
graph. A **resource profile** maps those names to executor and resource
settings for a particular deployment. The same workflow can therefore use
different settings on different deployments without changing the graph.

## Directory structure and selection

Each profile is a directory containing one TOML file per context. The filename
without `.toml` is the context name:

```text
resource_profiles/
├── local/
│   ├── cpu.toml
│   └── large.toml
└── cluster/
    ├── cpu.toml
    └── large.toml
```

For example, `resource_profiles/local/cpu.toml` might contain:

```toml
executor = "memory"
```

In another deployment, `resource_profiles/cluster/cpu.toml` could select an
HPC executor configured in the runtime:

```toml
executor = "slurm"
nodes = 1
cpu = 4
memory = "16Gi"
walltime = "01:00:00"
```

Select a profile with `runtime.start_new_run(..., profile="local")`
or through the CLI flag `tkr run ... --profile local`.

The runtime searches for a directory named after the selected profile in this
order:

1. `resource_profiles/` beside the discovered runtime configuration file,
   or in the current directory if no configuration file is found.
2. Paths listed in the runtime configuration's `[resources] paths` array.
   Relative paths are resolved against that same directory.
3. `~/.tierkreis/`.
4. Paths in `TKR_RESOURCE_PATH`, separated by the operating system's path
   separator (`:` on Unix).

The first matching profile directory wins; context files from later directories
are not merged. An unknown profile or requested context is an error. If a
task requests a named context, a profile must be selected.

## Using contexts in a graph

Pass `context` to `Graph.task` for a single task, or use `ExecutionContext`
to set defaults for nodes built in a scope. The following example uses
`cpu.toml` and `large.toml` from the selected profile. 

```python
from tierkreis.builder import ExecutionContext, Graph
from tierkreis.builtins import iadd
from tierkreis.data.models import TKR

# Build the nested workflow separately, without a context scope.
body = Graph(TKR[int], TKR[int])
inherited = body.task(iadd(body.inputs, body.const(1)))
overridden = body.task(iadd(inherited, body.const(1)), context="large")
body_workflow = body.finish_with_outputs(overridden)

graph = Graph(TKR[int], TKR[int])
with ExecutionContext("cpu"):
    result = graph.eval(body_workflow, graph.inputs)
    result = graph.task(iadd(result, graph.const(1)))
workflow = graph.finish_with_outputs(result)

# After saving the workflow with the runtime:
workflow_id = await runtime.save_workflow("example", workflow)
run_id = await runtime.start_new_run(workflow_id, {"value": 1}, profile="laptop")
```

The `eval` node receives `cpu` from the builder scope. At runtime, its nested
task without a context inherits `cpu`; the task explicitly marked `large`
uses `large` instead. The last task also uses `cpu`.

`Graph.eval`, `Graph.loop`, and `Graph.map` accept `context` and `executor`
and pass their defaults to nested tasks. A child node's explicit value takes
precedence over the inherited value. Nested `ExecutionContext` scopes work
similarly during graph construction: an inner scope overrides the values it
sets, and the outer scope is restored on exit. Context and executor are
inherited independently. An explicit or inherited executor overrides the
executor selected by the context's profile; otherwise the runtime's default
executor is used if the profile does not select one. An `executor` can also be
specified without a named context.

## Context options

Each profile context can set the following fields. 
They describe task requirements and executor-specific options. How each
setting is used depends on the selected executor.

| Option | Description |
| --- | --- |
| `executor` | Name of the configured executor, such as `"slurm"` or `"memory"`. |
| `nodes` | Number of nodes requested. |
| `cpu` | CPU cores requested per node. |
| `memory` | Memory requested per node, written in GiB (for example, `"16Gi"`). |
| `gpu` | GPUs requested per node. |
| `walltime` | Maximum run time in `HH:MM:SS` format, with at least one hour. The HPC executor defaults to `"01:00:00"`. |
| `queue` | Scheduler queue or partition. |
| `account` | Scheduler account or project. |
| `modules` | List of environment modules to load before running the worker. |
| `env` | Environment variables exported by the job script. |
| `gres` | List of scheduler-specific generic resource requests. |
| `qpus` | List of QPU identifiers to request. |
| `extra_scheduler_args` | Map of scheduler options to their string values. For valueless flags, use `extras.flags`. |
| `extras.flags` | List of valueless scheduler flags, for example `["--exclusive"]`. |
| `max_concurrency` | Positive concurrency limit; validated but not currently enforced by the orchestrator. |

All fields are optional. `nodes`, `gpu`, and `max_concurrency` must be positive
when specified. Profile validation accepts positive CPU quantities and
unit-bearing memory quantities; the HPC executor currently requires a numeric
`cpu` and an integer `Gi` memory string, and truncates fractional CPU counts.
For HPC jobs, specify `nodes` explicitly; an omitted value becomes `0`.
Executor resource limits are not currently enforced by the HPC executor.

The shared resource-profile format also accepts executor-specific
`native.<executor>` tables, but the HPC executor does not currently apply them.
Use `extra_scheduler_args` or `extras.flags` for scheduler-specific options.

Optional nested tables are also available:

| Table | Fields |
| --- | --- |
| `[user]` | `mail`: email address for scheduler notifications. |
| `[mpi]` | `proc`: total MPI processes as a string; `max_proc_per_node`: MPI processes per node as a string; `launcher`: command and arguments to run inside the allocation. Defaults to `["mpiexec"]`; set to `[]` to run without an MPI launcher. |
| `[container]` | `image`, `engine`, and `extra_args` (required; use an empty table if there are no extra options); `name` (optional container name); `env_file` (optional path to an environment file). Container settings are currently rendered by the PBS adapter. |

Scheduler support for individual fields varies. For example, the Slurm
template renders `gres` and `qpus`, while PBS and PJSUB have their own
resource-option syntax. Use `extra_scheduler_args` or `extras.flags` for
options not covered by the portable fields.
