# Tierkreis

Tierkreis describes workflows as typed Python graphs and executes them with a Rust runtime.

## Using Tierkreis

### Core Concepts

- A workflow is a typed, directed dataflow graph. Nodes represent values or work; edges carry values between nodes and encode dependencies.
- `TKR[T]` is a typed reference to a graph edge carrying a value of type `T`. Graph inputs and outputs are declared explicitly, and the finished `Workflow[Inputs, Outputs]` is the reusable graph definition.
- Evaluation is driven by requested outputs. A node can run once its inputs are available; lazy `ifelse` evaluates the selected branch, while eager `eifelse` can evaluate both branches.
- Worker tasks are the atomic units of user-defined work. The runtime schedules tasks to executors; nested workflows, loops, and maps express larger computations.
- A workflow run is an execution of a workflow with inputs. An attempt is a particular execution attempt; retries/restarts create a new attempt while preserving unaffected state.
- Nodes in nested graphs have locations. Loop/map locations identify iterations or elements and can be used to query traces.

### Define Graphs

- MUST define graphs with typed Python inputs and outputs, `TKR[T]` edge values, and an explicit `Workflow[Inputs, Outputs]` return type.
- Use typed worker task declarations when building a graph. Do not call worker implementations from graph-building code.
- Available node types: Input and Output; Const (`Graph.const`); IfElse (`Graph.ifelse`, lazy); EagerIfElse (`Graph.eifelse`); Task (`Graph.task`); Eval (`Graph.eval`); Loop (`Graph.loop`); Map (`Graph.map`); and Embed (`Graph.embed`). `Graph.graph_const` and `Graph.ref` are helpers for typed graph references, not separate node types.
- Use these builder operations instead of constructing graph serialization directly.
- See `docs/source/graphs/` and `docs/source/examples/` for graph semantics and runnable examples.

```python
from typing import NamedTuple

from tierkreis.builder import Graph
from tierkreis.builtins import iadd
from tierkreis.models import TKR, Workflow


class Inputs(NamedTuple):
        left: TKR[int]
        right: TKR[int]


class Outputs(NamedTuple):
        value: TKR[int]


def graph() -> Workflow[Inputs, Outputs]:
        builder: Graph[Inputs, Outputs] = Graph(Inputs, Outputs)
        total: TKR[int] = builder.task(
                iadd(a=builder.inputs.left, b=builder.inputs.right)
        )
        return builder.finish_with_outputs(Outputs(value=total))
```

### Write Workers

- A worker is a named namespace of atomic tasks. Keep its task API separate from its implementation: graph code imports typed task declarations, while the runtime launches the corresponding implementation through an executor.
- Give each worker a unique namespace and keep API and implementation task names, input names, and output types in agreement.
- Annotate every task input and return value. Use supported Tierkreis model types; use a typed `NamedTuple` when a task returns multiple named outputs.
- Keep worker implementations independent of graph construction. Do not invoke implementation functions directly from a graph or use untracked side effects as workflow outputs.
- Worker implementations may be Python or external programs. External workers must follow the worker-call contract and write declared outputs to the provided output locations.
- Report failures by raising an exception or returning a non-zero exit status. Use worker logging/stdout/stderr for diagnostics; only declared outputs are tracked as workflow results.
- Generate a worker scaffold with `tkr init worker --worker-name <name>`; use `--external` for a non-Python worker interface.
- Generate typed API stubs from worker definitions with `tkr init stubs`. By default, the CLI searches the worker directory and writes each API to `api/api.py`; use `--worker-directory` and `--api-file-name` to override these paths.
- Treat generated API files as build output. Import the generated task declarations in graph code and call them through `Graph.task`; do not duplicate their signatures by hand.

```python
from tierkreis import Worker

worker = Worker("arithmetic")


@worker.task()
def add(left: int, right: int) -> int:
        return left + right
```

### CLI

- Run a serialized workflow with `tkr run <workflow-file>`. Add `--print-output` (or `-o`) to print outputs and `--verbose` for verbose logging.
- Use `tkr exec` to start the runtime executor and `tkr serve` to start the visualization server. Use `tkr --help` for all commands and options.
- Initialize a worker with `tkr init worker --worker-name <name>`, then regenerate task stubs with `tkr init stubs` after changing task signatures.
- The CLI loads `tierkreis.toml` when present and otherwise uses the default runtime configuration.

### Inspect Workflows

- The server binds to port `3000`, uses SQLite runtime state, and serves the built visualization from `tierkreis_visualization/tierkreis_visualization/static/dist`.
- The REST API base is `/api`. Main endpoints include `GET /api/info`, `GET /api/workflows/`, `GET /api/workflows/{run_id}/graphs?locs=...`, `GET /api/workflows/{run_id}/nodes/{location}/outputs`, and `GET /api/workflows/{run_id}/nodes/{location}/inputs/{port}`. Node errors and logs are available under `/api/workflows/{run_id}/nodes/{location}/`.
- OpenAPI is available at `/api-docs/openapi.json`; Swagger UI is at `/swagger-ui`. Some route parameter names say `workflow_id`, while requests identify a run ID.
- Graph views support run/attempt selection, task restart, location-pattern trace queries, node search, charts, and run/node/executor monitoring.
- Loop/map location patterns support wildcards such as `N2.L*.N3.M*`. A pattern ending at a loop or map wildcard returns nested outputs; `N*` patterns are not supported.

### Configure Runtime and Resources

- `tierkreis.toml` configures Rust runtime storage, executors, defaults, and logging. It is separate from Python graph definitions.
- Configuration discovery checks `tierkreis.toml` in the current directory and its ancestors, then `TIERKREIS_CONFIG`, then `~/.tierkreis/tierkreis.toml`.
- Use the `type` values defined in `tierkreis/src/runtime.rs`. The example below uses in-memory runtime state; configure SQLite state when persistence is required.

```toml
default_storage_name = "file"
default_executor_name = "subprocess"

[asset_storage.memory]
type = "Memory"

[asset_storage.file]
type = "File"
asset_dir = ".tierkreis/assets"

[executors.memory]
type = "Memory"
output_storage_name = "memory"

[executors.subprocess]
type = "Subprocess"
subprocess_storage_name = "file"
output_storage_name = "file"

[runtime_state]
type = "Memory"

[logging_config]
log_file = ".tierkreis/tierkreis.log"
log_format = "compact"
log_level = "info"
service_name = "tierkreis"
```

- Select a resource profile such as `local` or `HPC` when running workflows. Profiles map task execution contexts to executors and resource settings.
- Put profile contexts in `profiles/<profile>/<context>.toml`. A profile may define the executor, resource requests, and environment variables:

```toml
# profiles/local/subprocess.toml
executor = "subprocess"
cpu = 2

[env]
TKR_PROFILE_MARKER = "from-runtime"
```

- Use `ExecutionContext("cpu_only")` to scope placement to a graph region; task declarations can select a context such as `context="subprocess"`. Nested graph nodes inherit the surrounding context. CPU/resource requests are primarily used by HPC executors.
- HPC execution is available through Slurm, PBS, and PJSUB backends. Verify scheduler-specific options against the executor configuration for the target cluster.

## Contributing to Tierkreis

### Setup and Checks

- Python tooling uses `uv` 0.12.3 or newer. `devenv` is optional and supplies the Rust/Python toolchains, `uv`, `just`, SQLite/Diesel, and frontend tooling.
- Run focused Rust tests with `cargo test -p tierkreis`; run the Python suite with `uv run pytest`. Common repository checks are `just test` and `just docs`.
- On macOS or when Rust linking fails with a missing `iconv`, run Rust commands inside `devenv shell`.
- Documentation sources are in `docs/source/` and use MyST/Sphinx. Do not edit generated docs or frontend assets unless regeneration is required.

### Code and Test Guidelines

- Read `README.md` and relevant docs before changing public behavior.
- Keep changes focused and follow the conventions of the owning module. Add or update tests for behavior changes.
- Keep graph examples fully typed and preserve `Workflow[Inputs, Outputs]` types across nested workflows and worker boundaries. Run `pyright` for graph API changes.
- Runtime restart creates a new attempt from a selected run and reruns the selected task locations and their dependants. Preserve unaffected node state and verify loop/map location handling when changing restart behavior.
- Lazy map evaluation must not eagerly evaluate mapped work that is not required by the workflow result.
- Keep generated docs and frontend bundles out of source changes unless the task explicitly requires regeneration.

### Project Layout

- Rust runtime and CLI: `tierkreis/src/`.
- Rust Axum server and routes: `tierkreis/src/server.rs` and `tierkreis/src/server/routes.rs`.
- Typed Python graph builder: `tierkreis/tierkreis/builder.py`.
- Visualization frontend: `tierkreis_visualization/`.
- Workers: `tierkreis_workers/`.
- Graph docs and examples: `docs/source/graphs/` and `docs/source/examples/`.

## Self-Review Checklist

- Are graph inputs, outputs, edges, and worker tasks fully typed?
- Did focused Rust/Python tests and relevant type checks pass?
- Do CLI, API, configuration, restart, and resource-profile behavior match the documented runtime contract?
- Are scheduler-specific settings and location-pattern edge cases covered where relevant?
- Are generated docs and frontend assets unchanged unless regeneration was required?
