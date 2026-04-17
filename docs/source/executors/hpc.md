# Tierkreis for HPC

If an HPC cluster provides a job submission system and a shared filesystem then we can use Tierkreis in the following way.

To take advantage of the HPC job submission system we use a specific implementation of the Tierkreis executor interface tailored to the system used in the cluster.
HPC Executors provide a simple interface to interact with submission systems of HPC Centers.
They work slightly differently compared to other executors as they wrap a command into a format accepted by the submission system.
The executors therefore only provide a bridging mechanism to transform the resource and job specification into a valid submission command.
On the requested resources, the command is treated as a regular submission with all the associated complexities.

To use the HPC cluster file system we just use the built-in Tierkreis `FileStorage` as usual.

## Job Spec

The `JobSpec` is a common description of the arguments required to submit a program to submission systems.
It provides a single entrypoint to any submission system without the need to rewrite job scripts at the cost of missing some custom flags.
Most configurations are optional, and will fall back to the configured default of the submission system.
An example configuration is given below.

- Job Name, to identify the job
- Command, the command to be run on the compute nodes
- Walltime, the maximum duration of the job of the format "HH:MM:SS"
- Output path, output files for log messages
- Error path, error path for log messages
- Queue, the queue or resource to submit to
- Account, the entity to be charged/associated with the job

Environment variables can be loaded through the submission systems mechanism when provided as dictionary of strings to the `environment` field.
Commands that are either not covered by the spec or are custom to the deployment can be provided as a `extra_scheduler_args` for which the key is the argument and the value its value.
Providing `None` can be used to supply flags like

```py
env = {"EDITOR": "/usr/bin/emacs"}
args = {
    "--set-arg": "1",
    "--set-flag": None,
}
```

Beyond the base configuration further configurations can be made through the subspecs.

### Resources

The following resources can be specified:

- Nodes, the number of nodes to use
- Cores per node, how many cpus are assigned to a node
- Memory, GBs of memory assigned to a node
- GPUs, number of GPUs assigned to a node

### User

- Mail: the user email, which if set up by the system will be used to signify job completion

### MPI

- Proc, the total number of processes
- Max proc per node, the maximum number of processes per node

### Containers

Currently not supported.

### Example

A sample job configuration is given below:

```py
JobSpec(
    job_name="test_job",
    command="/root/.local/bin/uv run /worker/main.py ",
    account="test_usr",
    resource=ResourceSpec(nodes=2, memory_gb=None),
    walltime="00:15:00",
    mpi=MpiSpec(max_proc_per_node=1),
    extra_scheduler_args={"--open-mode=append": None},
    output_path=Path("./logs.log"),
    error_path=Path("./errors.log"),
)
```

## Examples

Tierkreis is compatibble with many schdeduling systems.
On the following pages you can find system-specific information.

```{toctree}
:maxdepth: 1
:hidden:
./hpc/pjsub-fugaku.md
./hpc/psij_executors.md
```
