---
file_format: mystnb
kernelspec:
  name: python3
---

# Graph using built-in tasks

To create this graph we need only to install the `tierkreis` package:

```
pip install tierkreis
```

## Constructing the graph

First we instantiate a `Graph` object.
The arguments to the constructor describe the inputs and outputs of the graph respectively.
The following graph has no inputs and outputs a single integer.

```{code-cell} ipython3
from tierkreis.builder import Graph
from tierkreis.models import EmptyModel, TKR

g = Graph(EmptyModel, TKR[int])
```

In general a graph can have a multiple inputs and multiple outputs
but for now we keep things simple.

```{note}
In order to keep a clear separation between the types used in the Tierkreis graph and the types already present in the Python language we wrap the former with `TKR`.
(The `TKR[A]` wrapper type indicates that an edge in the graph contains a value of type `A`.)
```

We can add constants to a graph using `Graph.const`.

```{code-cell} ipython3
one = g.const(1)
two = g.const(2)
```

The constants will be added into the data structure defining the graph.
In particular if the graph is serialized then these constants will be hard-coded into that serialization.

We can add tasks using `Graph.task`:

```{code-cell} ipython3
from tierkreis.builtins import iadd

three = g.task(iadd(g.const(1), g.const(2)))
```

In this example we import the type stubs provided by the Tierkreis library for the built-in functions.
This allows us to use the [pyright](https://github.com/microsoft/pyright) static analysis tool to check that the input and outputs types of the tasks are what we expect them to be.

To finish, we convert the graph into a runnable `Workflow` by specifying the outputs:

```{code-cell} ipython3
workflow = g.finish_with_outputs(three)
```

## Running the graph

To run a general Tierkreis graph (`Workflow`) we need to set up:

- a way to store and share inputs and outputs (the 'storage' interface)
- a way to run tasks (the 'executor' interface)

For this example we use the `FileStorage` that is provided by the Tierkreis library itself.
The inputs and outputs will be stored in a directory on disk.
(By default the files are stored in `~/.tierkreis/checkpoints/<WORKFLOW_ID>`, where `<WORKFLOW_ID>` is a `UUID` identifying the workflow.)

```{code-cell} ipython3
from uuid import UUID
from tierkreis.storage import FileStorage

storage = FileStorage(UUID(int=99), name="My first graph")
```

If we have already run this example then there will already be files at this directory in the storage.
If we want to reuse the directory then run

```{code-cell} ipython3
storage.clean_graph_files()
```

to get a fresh area to work in.

Since we are just using the Tierkreis built-in tasks the executor will not actually be called.
As a placeholder we create a simple `ShellExecutor`, also provided by the Tierkreis library, which can run bash scripts in a specified directory.

```{code-cell} ipython3
from pathlib import Path
from tierkreis.executor import ShellExecutor

executor = ShellExecutor(Path("."), workflow_dir=storage.workflow_dir)
```

With the storage and executor specified we can now run a graph using `run_graph`:

```{code-cell} ipython3
from tierkreis import run_graph
from tierkreis.storage import read_outputs

run_graph(storage, executor, workflow, {})
print(read_outputs(workflow, storage))
```
