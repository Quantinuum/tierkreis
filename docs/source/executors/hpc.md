# Tierkreis on HPC systems

The Rust runtime can submit worker commands through PBS, PJSUB, or Slurm. The
runtime and compute nodes must share the configured file asset storage.

## Configuration Options

Executors are defined in `tierkreis.toml`

Configure an executor under `[executors.<name>]`:

| Option | Required | Description |
| --- | --- | --- |
| `type` | Yes | Must be `"Hpc"`. |
| `hpc_storage_name` | Yes | Registered asset storage used to stage inputs and outputs for compute nodes. It must be accessible from the scheduler jobs. |
| `output_storage_name` | Yes | Registered asset storage where completed outputs are written. |
| `scheduler` | Yes | Scheduler adapter: `"pbs"`, `"pjsub"`, or `"slurm"`. |
| `poll_interval_secs` | No | How often to check submitted jobs, in seconds. Defaults to `1`. |
| `worker_commands` | No | Table mapping worker namespaces to command argument arrays. If omitted for a worker, the executor uses `tkr-<namespace-with-hyphens>-worker`. |
| `resources` | Yes | Resources available to this executor; see the fields below. |

The `[executors.<name>.resources]` table accepts:
```{note}
The defined resources should define the __maximally__ available resources on the executor. 
```
| Option | Required | Description |
| --- | --- | --- |
| `nodes` | Yes | Number of nodes. |
| `cores_per_node` | No | CPU cores per node. |
| `memory_per_node_gb` | No | Memory per node, in GiB. |
| `gpus_per_node` | No | GPUs per node. |
| `qpus` | No | List of QPU identifiers. |
| `gres` | No | List of scheduler-specific generic resource requests. |




```{important}
In HPC environments it is common that environments differ between login and compute nodes.
Make sure that workers are available on compute nodes or provide `worker_commands`.
```

### Example: Slurm

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
```

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

```

Account,queue, and other Fugaku-specific PJSUB options belong in the profile's
`extra_scheduler_args` or `extras.flags`.

For example, use the following resource profile alongside the executor
configuration above:

```toml
executor = "pjsub"
nodes = 1
cpu = 48
walltime = "01:00:00"

[extras]
flags = ["--no-check-directory"]
```


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
