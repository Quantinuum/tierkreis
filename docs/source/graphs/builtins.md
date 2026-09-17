---
file_format: mystnb
kernelspec:
  name: python3
---

# Tasks: Using Builtins

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

The Rust-backed runtime owns workflow state, assets, and task execution. The
default runtime discovers the installed `tkr-builtins` worker on `PATH`.

```{code-cell} ipython3
from tierkreis import new_default

runtime = await new_default()
```

Save the workflow, start a run, wait for attempt `0`, and fetch the outputs:

```{code-cell} ipython3
with runtime:
    workflow_id = await runtime.save_workflow("My first graph", workflow)
    run_id = await runtime.start_new_run(workflow_id, {})
    await runtime.wait_for(run_id, 0)
    outputs = await runtime.get_outputs(run_id, 0)

print(outputs)
```
