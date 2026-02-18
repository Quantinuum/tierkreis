---
file_format: mystnb
kernelspec:
  name: python3
---

# Fugaku pjsub

We see how to run a worker using `pjsub` on Fugaku.
In particular we do not focus on the exact structure of our Tierkreis graph.
For a full working example please see the HPC graph in the [HPC example](../../examples/hpc.ipynb).

In this example we configure a `UvExecutor` as the default executor for small tasks and a `PJSUBExecutor` to run larger tasks using the `pjsub` job submission system.

First we take care of the storage layer by using the built-in `FileStorage`.
Since Fugaku mounts the home directory in login, pre-post and compute nodes we can leave the default checkpoints location as `~/.tierkreis/checkpoints`.

```{code-cell} ipython3
from uuid import UUID
from tierkreis.storage import FileStorage
workflow_id = UUID(int=101)
storage = FileStorage(workflow_id, name="pjsub_fugaku", do_cleanup=True)
```

If we assume that our workers are located in the folder `./worker_registry` then we can create an executor for small Python tasks as follows.

```{code-cell} ipython3
from pathlib import Path
from tierkreis.executor import UvExecutor
registry_path = Path("./worker_registry")
common_executor = UvExecutor(registry_path=registry_path, logs_path=storage.logs_path)
```

Next we define an executor to run large computations using the `PJSUBExecutor`.
It executes the given command using `pjsub` according to the prescription given in the `JobSpec`.
In the command, note that we instruct `uv` to create/reuse a non-standard directory for the worker venv.
This takes care of the difference in architecture between the login and compute nodes on Fugaku.

```{code-cell} ipython3
from tierkreis.hpc import JobSpec, ResourceSpec
from tierkreis.executor import PJSUBExecutor
pjsub_uv_spec = JobSpec(
    job_name="tkr_symbolic_ciruits",
    account="<Fugaku group name here>",
    command="env UV_PROJECT_ENVIRONMENT=compute_venv uv run main.py",
    resource=ResourceSpec(nodes=1, memory_gb=None, gpus_per_node=None),
    walltime="00:15:00",
    output_path=Path(storage.logs_path),
    error_path=Path(storage.logs_path),
    include_no_check_directory_flag=True,
)
pjsub_uv_executor = PJSUBExecutor(spec=pjsub_uv_spec, registry_path=registry_path, logs_path=storage.logs_path)
```

To bring this all together we use a `MultipleExecutor` to specify which executors get used for which workers.
The following code assigns `common_executor` as the default executor and uses the `pjsub_uv_executor` for all tasks belonging to the `compute_node_worker`.

```{code-cell} ipython3
from tierkreis.executor import MultipleExecutor
multi_executor = MultipleExecutor(
    common_executor,
    executors={"custom": pjsub_uv_executor},
    assignments={"compute_node_worker": "custom"},
)
```
