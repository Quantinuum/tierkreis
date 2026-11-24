# Runtime storage and executors

The Rust runtime constructs storage and executors from `tierkreis.toml`.
Workflows refer to execution contexts and resource profiles; callers do not
construct Python executor objects for each run.

```{toctree}
:maxdepth: 1
:hidden:
hpc.md
```

## Asset storage

Every graph input and task output is an asset. Runtime state records its
`AssetKey`; the selected `AssetStorage` transfers the serialized bytes to and
from executors. Available implementations are:

- `Memory`, for local tests and the in-memory executor;
- `File`, for subprocesses and schedulers that share a filesystem.

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

```toml
default_executor_name = "subprocess"

[executors.memory]
type = "Memory"
output_storage_name = "memory"

[executors.subprocess]
type = "Subprocess"
subprocess_storage_name = "file"
output_storage_name = "file"
```

Use graph metadata and resource profiles to select a non-default executor.
Profiles can also provide environment variables and resource requests, and a
profile can be selected for an individual run:

```python
run_id = await runtime.start_new_run(workflow_id, inputs, profile="gpu")
```

## Configuration

The default runtime already defines a `subprocess` executor using file-backed
assets. Override it when a deployment needs different named stores or a worker
does not install the conventional command:

```toml
[asset_storage.shared]
type = "File"
asset_dir = "/shared/tierkreis/assets"

[executors.subprocess] # subprocess is the name of the executor
type = "Subprocess"
subprocess_storage_name = "shared"
output_storage_name = "shared"

[executors.subprocess.worker_commands]
openssl_worker = ["/opt/workers/openssl_worker/main.sh"]
```

Worker inputs are copied from the executor's configured asset storage (`subprocess_storage_name`) before launch,
and outputs are saved to `output_storage_name` when the worker finishes. 

External workers can be supplied as an command invocation through `executors.subprocess.worker_commands`.
Each line is a worker name and an argument list, so wrappers can include fixed arguments,
for example `["uv", "run", "/opt/workers/example/main.py"]`.
