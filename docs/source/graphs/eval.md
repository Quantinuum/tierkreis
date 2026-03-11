---
file_format: mystnb
kernelspec:
  name: python3
---

# Eval: Using nested graphs

To create this graph we need only to install the `tierkreis` package:

```
pip install tierkreis
```

## Graph

We can run graphs from within other graphs by using `GraphBuilder.eval`.
Recall the `fib_step` graph that we wrote in the [previous tutorial](./loop.md)

```{code-cell} ipython3
from typing import NamedTuple

from tierkreis.builder import GraphBuilder
from tierkreis.builtins import iadd
from tierkreis.models import TKR


class FibData(NamedTuple):
    a: TKR[int]
    b: TKR[int]


fib_step = GraphBuilder(FibData, FibData)
sum = fib_step.task(iadd(fib_step.inputs.a, fib_step.inputs.b))
fib_step.outputs(FibData(fib_step.inputs.b, sum))
```

We create a graph `fib4` that calls `fib_step` three times.
The graph will have no inputs and gives a single integer as output:

```{code-cell} ipython3
from tierkreis.models import EmptyModel

fib4 = GraphBuilder(EmptyModel, TKR[int])
```

The `GraphBuilder.eval` method takes a `GraphBuilder` object as its first argument
and the appropriately typed input data as the second object.

```{code-cell} ipython3
second = fib4.eval(fib_step, FibData(a=fib4.const(0), b=fib4.const(1)))
```

We can iterate manually as follows:

```{code-cell} ipython3
third = fib4.eval(fib_step, second)
fourth = fib4.eval(fib_step, third)
fib4.outputs(fourth.b)
```

In the [next tutorial](./loop.md) we will see how to iterate programmatically.

## Execution

Since we still only use built-in functions, we execute the graph in the same way as before.

```{code-cell} ipython3
from uuid import UUID
from pathlib import Path

from tierkreis import run_graph
from tierkreis.storage import FileStorage, read_outputs
from tierkreis.executor import ShellExecutor

storage = FileStorage(UUID(int=99), name="Nested graphs using Eval")
executor = ShellExecutor(Path("."), workflow_dir=storage.workflow_dir)

storage.clean_graph_files()
run_graph(storage, executor, fib4.get_data(), {})
print(read_outputs(fib4, storage))
```
