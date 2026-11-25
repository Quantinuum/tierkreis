# Runtime Configuration

The runtime configuration specifies storage, executors, state, logging, and
resource-profile settings. Workflows refer to execution contexts defined by
resource profiles.

```{toctree}
:maxdepth: 1
:hidden:
contexts.md
hpc.md
```

## Configuration file

The runtime discovers `runtime.toml` in the current directory or its ancestors,
then checks `TKR_CONFIG`, then `~/.tierkreis/runtime.toml`. A file with another
name, such as `tierkreis.toml`, can be supplied explicitly or through
`TKR_CONFIG`. Python's `new_default()` uses built-in defaults rather than
loading this file; use `load_runtime_config()` and `new_from_config()` to load
it.

The following tables list the supported fields. Required fields must be
present when deserializing a configuration file.
#### Top-level fields

| Field | Required | Description |
| --- | --- | --- |
| `default_storage_name` | Yes | Name of an entry in `asset_storage`, used as the default asset store. |
| `default_executor_name` | Yes | Name of an entry in `executors`, used when no executor is selected for a task. |
| `asset_storage` | Yes | Named asset storage definitions. |
| `executors` | Yes | Named executor definitions. |
| `runtime_state` | Yes | Workflow state backend. |
| `logging_config` | No | Logging and telemetry settings. If omitted, the built-in logging defaults are used. |
| `resources` | No | Resource-profile selection and search paths. |

#### `[asset_storage.<name>]`

| Field | Required | Description |
| --- | --- | --- |
| `type` | Yes | `"Memory"` or `"File"`. |
| `asset_dir` | For `File` | Directory used for file-backed assets. |

#### `[executors.<name>]`

Every executor requires `type` and `output_storage_name`. The latter names a
registered asset store for its outputs. The remaining fields depend on
`type`:

| Type | Additional fields |
| --- | --- |
| `Memory` | None. |
| `Subprocess` | `subprocess_storage_name` (required asset store for staging); `worker_commands` (optional map of worker names to command argument arrays). |
| `Hpc` | `hpc_storage_name`, `scheduler`, and `resources` (required); `poll_interval_secs` (optional; defaults to `1`); `worker_commands` (optional). |
| `Nexus` | `client_config` (required table). |
| `Qrmi` | `config` (required table; available on Unix). |


#### `[runtime_state]`

| Field | Required | Description |
| --- | --- | --- |
| `type` | Yes | `"Memory"` or `"Sqlite"`. |
| `memory` | For `Sqlite` | If `true`, use an in-memory SQLite database; if `false`, use a persistent database. |
| `url` | No | SQLite database path or URL for persistent state. If omitted, `DATABASE_URL` is used, falling back to `~/.tierkreis/state.sqlite`. Ignored when `memory = true`. |

#### `[logging_config]`

| Field | Required | Description |
| --- | --- | --- |
| `log_format` | Yes, if the table is present | `"json"`, `"pretty"`, or `"compact"`. |
| `log_file` | No | Log file path; if omitted from this table, logs go to stderr. |
| `log_level` | No | Tierkreis log filter level; defaults to `"info"` when omitted. |
| `otel_endpoint` | No | OTLP endpoint for runtime traces and metrics. |
| `service_name` | No | OpenTelemetry service name; defaults to `"tierkreis"`. |
| `service_namespace` | No | OpenTelemetry service namespace; defaults to `"tierkreis"`. |

When the entire table is omitted, the built-in defaults use
`~/.tierkreis/tierkreis.log`, `"compact"`, and the
`OTEL_EXPORTER_OTLP_ENDPOINT` environment variable if set. If the table is
provided, set `otel_endpoint` explicitly to enable telemetry.

#### `[resources]`

| Field | Required | Description |
| --- | --- | --- |
| `environment` | No | Default resource profile name; takes precedence over `TKR_ENV`. |
| `paths` | No | Array of additional profile search roots. |

See [Execution context and profiles](contexts.md) for profile selection and
context fields, and [Tierkreis on HPC systems](hpc.md) for HPC examples.



## Asset storage

Every graph input and task output is an asset. Runtime state records its
`AssetKey`; the selected `AssetStorage` transfers the serialized bytes to and
from executors. Available implementations are:

- `Memory`, for local tests and the in-memory executor;
- `File`, for subprocesses and schedulers that share a filesystem.

### Configuration Example

Storage is named, allowing an executor to name both the store from which it
loads inputs and the store to which it writes results.

```toml
default_storage_name = "file"

[asset_storage.file]
type = "File"
asset_dir = "/shared/tierkreis/assets"
```

## Executors

The runtime currently supports:

| Type | Use |
| --- | --- |
| `Memory` | Builtins in the runtime process |
| `Subprocess` | Installed `tkr-*-worker` commands |
| `Nexus` | Nexus-hosted jobs |
| `Qrmi` | QRMI execution |
| `Hpc` | PBS, PJSUB, or Slurm submission |

The default configuration combines a memory executor for builtins with a
subprocess executor for installed workers. A subprocess worker is normally
discovered as `tkr-<namespace>-worker`.
The subprocess executor launches installed worker commands in a isolated shell subprocess.

Use graph metadata and resource profiles to select a non-default executor.
Profiles can also provide environment variables and resource requests, and a
profile can be selected for an individual run:

```python
run_id = await runtime.start_new_run(workflow_id, inputs, profile="gpu")
```

### Configurations

Many of the executors provide custom configuration options:

For `[executors.<name>.client_config]` on a `Nexus` executor:

| Field | Required | Description |
| --- | --- | --- |
| `tls_mode` | Yes | `"Default"` for HTTPS. |
| `host` | Yes | Nexus host name. |
| `token_dir` | No | Directory containing Nexus authentication tokens; defaults to `~/.qnx/auth`. |

For `[executors.<name>.config]` on a `Qrmi` executor:

| Field | Required | Description |
| --- | --- | --- |
| `resource_id` | Yes | Resource identifier or backend name. |
| `resource_type` | Yes | One of `"ibm-quantum-system"`, `"qiskit-runtime-service"`, `"ibm-quantum-compute-service"`, `"pasqal-cloud"`, `"pasqal-local"`, `"alice-bob-felis"`, `"iqm-server"`, or `"oqtopus"`. |
| `environment` | No | Map of QRMI settings without the resource-name prefix. If empty, QRMI reads from the process environment. |
| `poll_interval_secs` | No | Status polling interval in seconds; defaults to `1` and must be positive. |

For `Hpc`, `scheduler` is `"pbs"`, `"pjsub"`, or `"slurm"`.
The `[executors.<name>.resources]` can be found [here](./hpc.md)

### Configuration Example

The default runtime already defines a `subprocess` executor using file-backed
assets. Override it when a deployment needs different named stores or a worker
does not install the conventional command:

```toml
default_executor_name = "subprocess"

[executors.memory]
type = "Memory"
output_storage_name = "memory"

[executors.subprocess]
type = "Subprocess"
subprocess_storage_name = "file"
output_storage_name = "file"

[executors.subprocess.worker_commands]
openssl_worker = ["/opt/workers/openssl_worker/main.sh"]
```

Worker inputs are copied from the executor's configured asset storage (`subprocess_storage_name`) before launch,
and outputs are saved to `output_storage_name` when the worker finishes. 

External workers can be supplied as an command invocation through `executors.subprocess.worker_commands`.
Each line is a worker name and an argument list, so wrappers can include fixed arguments,
for example `["uv", "run", "/opt/workers/example/main.py"]`.
