# Fugaku: PJSUB

Fugaku can run Tierkreis tasks through the Rust runtime's `Hpc` executor. Put
the asset directory and SQLite state database on a filesystem visible to the
login, pre/post, and compute nodes.

```toml
default_storage_name = "fugaku"
default_executor_name = "local"

[asset_storage.fugaku]
type = "File"
asset_dir = "/shared/tierkreis/assets"

[executors.local]
type = "Subprocess"
subprocess_storage_name = "fugaku"
output_storage_name = "fugaku"

[executors.pjsub]
type = "Hpc"
hpc_storage_name = "fugaku"
output_storage_name = "fugaku"
scheduler = "pjsub"
poll_interval_secs = 10

[executors.pjsub.worker_commands]
compute_node_worker = ["uv", "run", "/shared/workers/compute/main.py"]

[executors.pjsub.resources]
nodes = 1
cores_per_node = 48
gpus_per_node = 0

[runtime_state]
type = "Sqlite"
memory = false
url = "/shared/tierkreis/state.db"
```

Put site- and workflow-specific requirements in a resource profile such as
`resource_profiles/fugaku/compute.toml`:

```toml
executor = "pjsub"
nodes = 1
cpu = 48
walltime = "00:15:00"

[env]
UV_PROJECT_ENVIRONMENT = "compute_venv"

[extra_scheduler_args]
"--no-check-directory" = true
```

Select the profile on the workflow or when starting the run. Tasks without an
HPC context continue to use the default local subprocess executor. Account,
queue, and other Fugaku-specific PJSUB options belong in the profile's native
or extra scheduler arguments.
