# Runtime storage and executors

The Rust runtime constructs storage and executors from `tierkreis.toml`.
Workflows refer to execution contexts and resource profiles; callers do not
construct Python executor objects for each run.

```{toctree}
:maxdepth: 1
:hidden:
shell.md
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

## Legacy executor APIs

`UvExecutor`, `ShellExecutor`, `MultipleExecutor`, `TaskExecutor`, `StdInOut`,
and PSI/J belong to the legacy Python controller. The Rust runtime replaces
their routing behavior with named executor configuration and worker commands.
There is not yet a Rust-runtime equivalent for an arbitrary `script()` node
whose input and output are redirected through stdin and stdout.
