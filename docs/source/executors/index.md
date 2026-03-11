# Executors

Executors are fundamental in running graph nodes in different environments.
They also ensure that the workers can fulfill their contracts by preparing inputs and outputs.

```{toctree}
:maxdepth: 1
shell.md
hpc.md
hpc/index
```

## The worker contract

To function properly, the controller makes assumptions about the input and outputs of tasks.
Typically the workers assure that these assumptions are met.
The executors have to provide the required information to ensure the workers do this correctly.
The information is stored in the [`WorkerCallArgs`](#tierkreis.controller.data.location.WorkerCallArgs) which is also used to invoke the worker.
If a worker cannot produce the necessary files, this causes tierkreis to not complete a workflow.
The responsibility of the executor is to ensure the normal operations by, for example, translating relative paths to absolute ones.

### Controller files

The controller is aware of the following files, if not specified otherwise they are expected in `<checkpoints_dir>/<workflow_id>/<node_location>/`

- The `logs` files: any logging information should be written to either of two files
  - controller logs are written to `<checkpoints_dir>/<workflow_id>/logs` and contain global progress information
  - worker specific logs should be written to the `logs_path` location of its call arguments, typically `<checkpoints_dir>/<workflow_id>/<node_location>/logs`
- The presence of the `nodedef` file indicates the node has been started, workers should not interact with this
- The `definition` file contains the serialized `WorkerCallArgs`, the worker needs to parse this to find out about the locations inputs, outputs and the here listed files.
- Completion is indicated by the `_done` file, workers must set this once they have written all outputs
- Failures is indicated by the `_error` file, workers must set this if they can not complete normal execution
- In case of failure error messages should be written to the `errors_path` location of its call arguments, typically `<checkpoints_dir>/<workflow_id>/<node_location>/logs`.
  Currently as a fallback it is also possible to write to the `<checkpoints_dir>/<workflow_id>/<node_location>/_errors` file.

### Task, Inputs and Outputs

`WorkerCallArgs` contain the information of the function name of the task to call and it's inputs and the location to write outputs to.
To supply workers with their inputs the `WorkerCallArgs` specify a mapping of input name to a location where the input is stored.
For example, the `greet` task of the [`hello_world_worker`](../worker/hello_world.md) expects two string inputs and outputs one file.
The inputs are can be looked up by port name (`greeting`, `subject`) their values are stored in the output of other nodes in a a file `<checkpoints_dir>/<workflow_id>/<node_location>/outputs/<port_name>`.
The outputs of a worker follow the same pattern and are stored in the `output_dir` directory specified in the call args.
For each value to output, there is an entry for it in the caller arguments output mapping.

## Worker Registries

Executors are dispatching other programs.
To do this, they need to know where they can find the programs to run.
Most executors will take a `registry_path` argument which specifies a directory or list of directories.
At runtime, the executor will search the provided directories for the workers according to the task names.
For example assume the following directory structure (auxiliary files omitted):

```
project_root/
├── workers/
│  ├── first_worker/
│  │        ├── api/
│  │        │   └── stubs.py
│  │        └── main.py
│  └──second_worker/
│           ├── api/
│           │   └── stubs.py
│           └── main.py
├── alt_workers/
│  └── first_worker/
│           ├── api/
│           │   └── stubs.py
│           └── main.py
└── main.py
```

Where both workers `first_worker` implement the same api but provide different implementations.
Now as graph definition in `main.py`:

```py
from first_worker import some_task

graph = Graph(TKR[NoneType], TKR[NoneType])
graph.task(some_task)
```

Using the `UvExecutor` in the controller:

```py
executor = UvExecutor(
    logs_path=storage.logs_path
    registry_path=[
      Path(__file__).parent / "workers",
      Path(__file__).parent / "alt_workers",
    ]
)
```

will now search the provided directories `workers` and `alt_workers` for a directory called `first_worker` (inferred from the worker name in the stubs).
In the above scenario this would resolve to running `workers/first_worker/main.py`.
Removing the `Path(__file__).parent / "workers"` would then switch to `alt_workers/first_worker/main.py`.

## Existing executors

Currently the following basic executors are available:

| Executor         |             Target Workers              | Enabled |                                        Notes                                         |                                    Docs                                    |
| ---------------- | :-------------------------------------: | :-----: | :----------------------------------------------------------------------------------: | :------------------------------------------------------------------------: |
| UVExecutor       | Python based with dependency management |    ✔    |                           Default for python based workers                           |       [link](#tierkreis.controller.executor.uv_executor.UvExecutor)        |
| ShellExecutor    |                 Scripts                 |    ✔    |                                  Runs shell scripts                                  |    [link](#tierkreis.controller.executor.shell_executor.ShellExecutor)     |
| StdInOut         |                 Scripts                 |    ✔    |           Runs shell scripts or builtins with single input and output file           |          [link](#tierkreis.controller.executor.stdinout.StdInOut)          |
| InMemoryExecutor |                 Python                  |    ✔    | Runs in the same memory spaces as the controller, does not work for external workers | [link](#tierkreis.controller.executor.in_memory_executor.InMemoryExecutor) |
| SLURMExecutor    |                   Any                   |    ✔    |                        Wraps a command in a SLURM submission                         |       [link](#tierkreis.controller.executor.hpc.slurm.SLURMExecutor)       |
| PJSUBExecutor    |                   Any                   |    ✔    |                        Wraps a command in a PJSUB submission                         |       [link](#tierkreis.controller.executor.hpc.pjsub.PJSUBExecutor)       |
| PBSExecutor      |                   Any                   |   ❌    |                         Wraps a command in a PBS submission                          |                                                                            |

## Combining Executors

By default only one executor can be assigned to the controller.
All workers will be run through this executor.
To solve this you can provide an executor

If different executors are necessary they can be combined using the [`MultipleExecutor`](#tierkreis.controller.executor.multiple.MultipleExecutor) like so:

```py
def multiple_graph():
    # Both tasks are the same, we just use different names to test the task executor
    g = Graph(TKR[str], TKR[str])
    first_call = g.data.func(
        "shell_worker.meet",
        {"greeting": g.inputs.value_ref()},
    )
    out: TKR[str] = TKR(*first_call("value"))
    second_call = g.data.func(
        "stdinout_worker.greet",
        {"greeting": out.value_ref()},
    )
    output: TKR[str] = TKR(*second_call("value"))
    return g.finish_with_outputs(output)


def main():
    g = multiple_graph()
    storage = ControllerFileStorage(UUID(int=306), name="Multiple")
    first = ShellExecutor(
        WORKER_PATH,
        workflow_dir=storage.workflow_dir,
        env={"TEST_FLAG": "beautiful"},
    )
    second = StdInOut(
        WORKER_PATH,
        workflow_dir=storage.workflow_dir,
    )
    executor = MultipleExecutor(
        first, {"second": second}, {"stdinout_worker": "second"}
    )
    storage.clean_graph_files()
    run_graph(storage, executor, g, {"value": "world"})
```

It provides a an assignment of workers to executors by a string mapping, a default executor will execute all unassigned workers.
Alternativerly if you need control on task level, e.g., to execute the same worker in different environments you can use the [`TaskExecutor`](#tierkreis.controller.executor.task_executor.TaskExecutor):

```py
def task_graph():
    # Both tasks are the same, we just use different names to test the task executor
    g = Graph(TKR[str], TKR[str])
    first_call = g.data.func(
        "shell_worker.meet",
        {"greeting": g.inputs.value_ref()},
    )
    out: TKR[str] = TKR(*first_call("value"))
    second_call = g.data.func(
        "shell_worker.greet",
        {"greeting": out.value_ref()},
    )
    output: TKR[str] = TKR(*second_call("value"))
    return g.finish_with_outputs(output)

def main():
    g = task_graph()
    storage = ControllerFileStorage(UUID(int=305), name="Task")
    first = ShellExecutor(
        WORKER_PATH,
        workflow_dir=storage.workflow_dir,
        env={"TEST_FLAG": "cruel"},
    )
    second = ShellExecutor(
        WORKER_PATH,
        workflow_dir=storage.workflow_dir,
        env={"TEST_FLAG": "Goodbye"},
    )
    executor = TaskExecutor(
        {"shell_worker.meet": first, "shell_worker.greet": second}, storage
    )
    storage.clean_graph_files()
    run_graph(storage, executor, g, {"value": "world"})
```

## Writing your own executor

Most use cases should already be covered by the existing executors.
In some instances it might be necessary to write a custom executor which can be done by implementing the protocol.
The executor protocol defines a single function

```py
def run(self, launcher_name: str, worker_call_args_path: Path) -> None:
    ...
```

- `launcher_name` identifies the worker to be run
- `worker_call_args_path` is the _relative_ path to the node definition containing
  - the name of the worker task
  - _relative_ locations to input and output files
  - _relative_ locations to internal files, such as error flags and log files

The executor is responsible to modify these inputs in such a way that the worker can operate correctly.
Assuming the worker already can deal with all of this, a minimal executor could simply spawn a new process running the worker.

```py
class MinimalExecutor:
    def run(self, launcher_name: str, worker_call_args_path: Path) -> None:
        subprocess.Popen(
            [launcher_name, str(worker_call_args_path)],
        )
```
