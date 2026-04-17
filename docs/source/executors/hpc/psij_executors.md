# PSI/J Executors

[PSI/J](https://exaworks.org/psij-python/) is a unified submission interface for many schedulers.
Tierkreis is largely compatible with the `psij` python package.
It follows a similar design philosophy:
1. Define a jobs resources
2. Declare the executable you want to run
3. Instantiate the orchestrate
4. Run the job
5. Retrieve the results

## Using the PSI/J executor as drop in replacement

Assume we have a graph that we want to run on a cluster, e.g. using the `PBS` submission system.
```python
def mpi_graph() -> Workflow:
    builder = Graph(outputs_type=TKR[str])
    mpi_result = builder.task(mpi_task())
    return builder.finish_with_outputs(mpi_result)
```
In this case `mpi_task()` is a stub for an arbitrary task `None -> str`.
To run this on a cluster you have to provide a resource definition.
Using the Tierkreis functionality, you would use `JobSpec` in conjunction with `ResourceSpec`

```python
def pbs_spec() -> JobSpec:
    return JobSpec(
        job_name="test_job",
        account="pbsuser",
        command=("/home/pbsuser/.local/bin/uv run /mpi_worker/main.py"),
        resource=ResourceSpec(nodes=2, memory_gb=None, gpus_per_node=None),
        walltime="00:15:00",
        mpi=MpiSpec(max_proc_per_node=1),
        extra_scheduler_args={"-l place=scatter": None},
    )
```

in conjunction with a specific executor:
```python
executor = PBSExecutor(
    spec=pbs_spec(),
    registry_path=None,
    logs_path=storage.logs_path,
    command="qsub",
)
```

In principle, you can use the same specification for the `PSIJExecutor`.
```{important}
`tierkreis.JobSpec` and `psij.JobSpec` are not congruent. For more information see the [](#caveats)
```

```python
executor = PSIJExecutor(
    spec=pbs_spec(),
    launchers_path=None,
    logs_path=storage.logs_path,
    psij_executor="pbs",
)
```
Here the `psij_executor` argument will be forwarded to a `psij.JobExecutor` without validation. 

## Converting between jobs

Inside Tierkreis, we canonically use the `tierkreis.JobSpec` so in isolation you would not need the `psij.JobSpec`.
Still there are two conversion functions:
- [](#tierkreis.controller.executor.hpc.psij_conversion.psij_to_spec) converting `psij.JobSpec` -> `tierkreis.JobSpec`
- [](#tierkreis.controller.executor.hpc.psij_conversion.spec_to_psij) converting `tierkreis.JobSpec` -> `psij.JobSpec`

They try to match the arguments as closely as possible, but they are not inverse `psij_to_spec(spec_to_psij(X)) != X` especiilly when `extra_scheduler_args` or `psij.JobAttributes.custom_args` are set.

## Caveats

For most arguments `psij.JobSpec` and `tierkreis.JobSpec` and their respective executors overlap.
Here we list the known cases where they are not congruent:

### Executors

The main difference is:
- Tierkreis uses scheduler specific executor classes
- You cannot provide a path to the scheduler command binary for `psij`. The command has to be in `$PATH`.
- The `launchers_path` argument has no effect on the `PSIJExecutor`. We reommend using fully qualified paths for commands, e.g. `/home/pbsuser/.local/bin/uv run /mpi_worker/main.py` (for running on the compute nodes).
- `psij` uses an internal script to launch jobs, this can cause jobs to not launch identically to corresponding tierkreis executors

### Specs

- Tierkreis uses memory in `Gb`, `psij` in `b` (covered by the conversion)
- `psij` doesn't support containerizaiton
- `psij`uses `mpirun` as a launcher and automatically adds placement; Tierkreis uses `mpiexec` users need to add placement as an additional argument
- `extra_scheduler_args` are mostly incompatible with `psij.JobAttributes.custom_args`. Conversion will do a best effor job but can't account for renaming.

