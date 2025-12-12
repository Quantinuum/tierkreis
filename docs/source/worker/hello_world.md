---
file_format: mystnb
kernelspec:
  name: python3
---

# Hello world worker

We see how to run tasks that are not Tierkreis built-in tasks.
Although we can build workers that manually read and write to the appropriate files,
in this section we use the helper functions in the Tierkreis Python library.

## Worker code

Our worker will consist of a single function returning a string.
We first instantiate a `Worker` class, which is constructed using the name of the worker.
The name of the worker tells the executor which worker a task comes from.
Therefore all of the different workers used in a single graph should have distinct names.

```{code} ipython3
from tierkreis import Worker

worker = Worker("hello_world_worker")
```

The `Worker.task` method is used as a decorator to add functions to the worker.

```{code} ipython3
@worker.task()
def greet(greeting: str, subject: str) -> str: ...
```

```{note}
There are restrictions on the types of functions that `Worker.task` will accept.
If the function arguments and return types correspond to JSON types then the function will be accepted.
In addition there are a few ways of using more complex data structures,
but for now we keep things simple.
For more information please see [Complex types in Tierkreis Python workers](complex_types.md).
```

We use the `Worker.app` method to turn our Python programme into a legitimate Tierkreis worker.

```{code} ipython3
from sys import argv

if __name__ == "__main__":
    worker.app(argv)
```

The complete worker file is as follows:

```{literalinclude} ../../../examples/example_workers/hello_world_worker/main.py
:language: python
```

### Logging and Errors

The Tierkreis controller will automatically collect logs and errors by redirecting any output to `stdout` to `logs_path` and `stderr` to `errors_path` automatically.
Any other output, e.g. writing debug information to a log file won't be captured.
For convenience you can use the builtin logging functionality `worker.logger` which is an instance of a `logging.Logger`.
Raising an exception in a worker task will terminate the graph execution.
An example of this can be found in the examples: `error_handling_graph.py`

## Generating stubs

Since this worker uses the Tierkreis Python library, we can automatically generate stub files using the following command.

```{code-cell}
!cd ../../../examples/example_workers/hello_world_worker && uv run main.py --stubs-path ../../../docs/source/worker/hello_stubs.py > /dev/null 2>&1
```

## Graph creation

Now can we import the `greet` function from the stubs file and use it in `GraphBuilder.task`.

```{code-cell} ipython3
from tierkreis.builder import GraphBuilder
from tierkreis.models import TKR
from hello_stubs import greet

g = GraphBuilder(TKR[str], TKR[str])
output = g.task(greet(greeting=g.const("hello "), subject=g.inputs))
g.outputs(output)
```

## Execution

We use the `FileStorage` as usual but this time use the `UvExecutor` instead of the `ShellExecutor`.
To configure the `UvExecutor` we provide a path to a 'registry' folder of workers constructed using `uv`.
To add a worker to the registry we create a sub-folder of the registry folder containing a `main.py` that is executable by `uv`.
(The directory could be a whole `uv` project with a `pyproject.toml` or it might be that the `main.py` is a `uv` script.)
The folder name should correspond to the name of the worker.

```{code-cell}
from uuid import UUID
from pathlib import Path

from tierkreis import run_graph
from tierkreis.executor import UvExecutor
from tierkreis.storage import FileStorage, read_outputs

storage = FileStorage(UUID(int=99), "hello_world_tutorial", do_cleanup=True)
executor = UvExecutor(
    registry_path=Path("../../../examples/example_workers"), logs_path=storage.logs_path
)
run_graph(storage, executor, g.data, "world!")
read_outputs(g, storage)
```
