# Tierkreis on HPC systems

The Rust runtime can submit worker commands through PBS, PJSUB, or Slurm. The
runtime and compute nodes must share the configured file asset storage.

```toml
default_storage_name = "file"
default_executor_name = "slurm"

[asset_storage.file]
type = "File"
asset_dir = "/shared/tierkreis/assets"

[executors.slurm]
type = "Hpc"
hpc_storage_name = "file"
output_storage_name = "file"
scheduler = "slurm"
poll_interval_secs = 5

[executors.slurm.worker_commands]
mpi_worker = ["uv", "run", "/shared/workers/mpi_worker/main.py"]

[executors.slurm.resources]
nodes = 2
cores_per_node = 8
gpus_per_node = 0

[runtime_state]
type = "Sqlite"
memory = false
url = "/shared/tierkreis/state.db"
```

Scheduler resources are deployment limits. Per-workflow requirements belong in
a resource profile, for example `resource_profiles/cluster/hpc.toml`:

```toml
executor = "slurm"
nodes = 2
cpu = 8
walltime = "01:00:00"

[env]
OMP_NUM_THREADS = "8"
```

Select the profile with `new_default("cluster")`, the `TKR_ENV` environment
variable, or `profile="cluster"` on `start_new_run`. Scheduler-specific queue,
account, MPI, and extra arguments can also be supplied by the profile.

The legacy PSI/J executor is not supported by the Rust runtime. Existing PSI/J
documentation is retained only as legacy reference and should not be used for
new workflows.
