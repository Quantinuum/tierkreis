---
file_format: mystnb
kernelspec:
  name: python3
---

# Graph inputs and outputs

## Single input and single output

### Elementary types

Like Python functions, Tierkreis graphs can have input and output arguments.
We use the constructor of `Graph` to indicate that our function takes a single integer to a single integer:

```{code-cell} ipython3
from tierkreis.builder import Graph
from tierkreis.models import TKR

# f(x) = 2x + 1
f = Graph(TKR[int], TKR[int])
```

The implementation of this graph can be done entirely using Tierkreis built-in functions:

```{code-cell} ipython3
from tierkreis.builtins import iadd, itimes

double = f.task(itimes(f.const(2), f.inputs))
f_out = f.task(iadd(double, f.const(1)))
f = f.finish_with_outputs(f_out)
```

### Nested types within a single output

Sometimes we want to return a nested data structure within a single output.
To do this we can use a Python `NamedTuple`.
In practice these `NamedTuple`s will often be defined already in the API definition of a Tierkreis worker and so we usually import them rather than define them manually.

```{code-cell} ipython3
from typing import NamedTuple

class FibDataStruct(NamedTuple):
    a: int
    b: int
```

To use this as part of the signature of a graph, we wrap it in `TKR`.
The `TKR[A]` wrapper type indicates that a single input/output contains a value of type `A`.
The contents of `A` will not in general be accessible to the graph builder code.

```{code-cell} ipython3
from tierkreis.models import EmptyModel

init_data = Graph(EmptyModel, TKR[FibDataStruct])
init_workflow = init_data.finish_with_outputs(init_data.const(FibDataStruct(a=0, b=1)))
```

## Multiple inputs and multiple outputs

However a Tierkreis graph can also have multiple inputs and multiple outputs.
To indicate that more than one input/output is required we again use a `NamedTuple`,
except this time one whose attributes are all Tierkreis types (i.e. wrapped in `TKR`).

```{code-cell} ipython3
class FibData(NamedTuple):
    a: TKR[int]
    b: TKR[int]
```

To use this in the signature of a graph, we pass it directly in.
This way Tierkreis will interpret the different attributes of the `NamedTuple` as different inputs/outputs.

```{code-cell} ipython3
from tierkreis.builder import Graph
from tierkreis.builtins import iadd
from tierkreis.models import TKR

fib_step = Graph(FibData, FibData)
sum = fib_step.task(iadd(fib_step.inputs.a, fib_step.inputs.b))
fib_step = fib_step.finish_with_outputs(FibData(fib_step.inputs.b, sum))
```

Note that we are now able to access the contents of `FibData` in the graph builder.

```{note}
What would happen if we used a nested data structure inside a single input/output to construct this graph?
```

If instead we wanted to have a single output containing a nested structure `FibData`
then we would initialize the graph builder as follows:

```{code-cell} ipython3
class FibData(NamedTuple):
    a: int
    b: int

fib_step_2 = Graph(TKR[FibData], TKR[FibData])
```

However we would then not be able to access attributes of `FibData` in the graph builder code.

```{code} ipython3
# type error: 'TKR' object has no attribute 'a'
sum = fib_step_2.task(iadd(fib_step_2.inputs.a, fib_step_2.inputs.b))
```

```{hint}
We can use the different behavior of the above two examples to create a separation of concerns between the graph builder and the workers.
If some data is required in graph builder code then we use multiple inputs/outputs.
If some data is only used in workers and can be passed between them without the graph needing to inspect them then we use a single input/output containing within it a nested data structure.
```

## Combinations of single and multiple inputs

We can combine the various types of inputs and outputs in the natural way.
For instance the following are all valid ways to construct a `Graph` object:

```{code-cell} ipython3
class MultiPortInputData(NamedTuple):
    a: TKR[int]
    b: TKR[str]

class MultiPortOutputData(NamedTuple):
    a: TKR[str]
    b: TKR[list[int]]

g = Graph(TKR[int], TKR[str])
g = Graph(MultiPortInputData, MultiPortOutputData)
g = Graph(TKR[str], MultiPortOutputData)
g = Graph(MultiPortInputData, TKR[str])
```

## Execution

Since we still only use built-in functions, we execute the graph in the same way as before.
For the examples with graph inputs, we provide the input in the third argument of `run_graph`.

```{code-cell} ipython3
from uuid import UUID
from pathlib import Path

from tierkreis import run_graph
from tierkreis.storage import FileStorage, read_outputs
from tierkreis.executor import ShellExecutor

storage = FileStorage(UUID(int=99), name="Graph inputs and outputs")
executor = ShellExecutor(Path("."), workflow_dir=storage.workflow_dir)

storage.clean_graph_files()
run_graph(storage, executor, f, 10)
print(read_outputs(f, storage))

storage.clean_graph_files()
run_graph(storage, executor, init_workflow, {})
print(read_outputs(init_workflow, storage))

storage.clean_graph_files()
run_graph(storage, executor, fib_step, {'a': 0, 'b': 1})
print(read_outputs(fib_step, storage))
```
