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


## Fugaku: PJSUB

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


## MPI

An `mpi` table in the resource profile requests MPI ranks from the scheduler
and, by default, wraps the worker command with `mpiexec`:

```toml
executor = "slurm"
nodes = 1

[mpi]
proc = "4"
max_proc_per_node = "4"
# Optional: defaults to ["mpiexec"].
launcher = ["srun", "--mpi=pmix"]
```

The launcher runs **inside the batch script**, after the scheduler has
allocated the resources; it is not used to launch `sbatch`, `qsub`, or `pjsub`.
For example, the Slurm profile above produces `#SBATCH --ntasks=4` and a
command of the form:

```sh
srun --mpi=pmix /path/to/worker /path/to/worker-call.json
```

Use the launcher recommended by the cluster: Slurm may use `srun` with a
compatible PMI/PMIx plugin, while PBS commonly uses a scheduler-integrated
`mpiexec` or `mpirun`. Such launchers obtain process placement from the batch
allocation. Configure the site's PBS MPI allocation using
`max_proc_per_node` and, where needed, `extra_scheduler_args`; `proc` is
rendered as a total task request for Slurm and PJSUB. Additional launcher
options can be supplied as separate words in `launcher`. Each word is
shell-quoted when the script is rendered.

The worker binary must implement the Tierkreis worker protocol itself,
including input loading and output writing. It runs on every MPI rank and
must coordinate any shared-file access, typically writing results on rank 0.
No wrapper or Tierkreis MPI environment variables are needed. Do not start a
second MPI launcher inside the worker.

Set `launcher = []` to run the worker command without an MPI launcher.

The legacy PSI/J executor is not supported by the Rust runtime. Existing PSI/J
documentation is retained only as legacy reference and should not be used for
new workflows.
