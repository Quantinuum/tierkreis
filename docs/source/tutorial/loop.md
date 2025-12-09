---
file_format: mystnb
kernelspec:
  name: python3
---

# Iteration using Loop

One way to perform iteration in Tierkreis is to use `GraphBuilder.loop`.
The first argument to `GraphBuilder.loop` is a graph that constitutes the loop body.
The second is the initial input to the loop.

## Graphs

### Bounded iteration

As an example to introduce the concepts, we create a bounded iteration that has additional inputs to configure the iteration.
The signature of the loop body is:

```{code-cell} ipython3
from typing import NamedTuple
from tierkreis.builder import GraphBuilder
from tierkreis.models import TKR

class LoopBodyInput(NamedTuple):
    i: TKR[int]
    value: TKR[int]
    step: TKR[int]
    bound: TKR[int]

class LoopBodyOutput(NamedTuple):
    i: TKR[int]
    value: TKR[int]
    should_continue: TKR[bool]

loop_body = GraphBuilder(LoopBodyInput, LoopBodyOutput)
```

The loop body input type consists of all the variables that we want the loop body to have access to.
This includes variables that the loop does not modify, like `step` and `bound`.
The loop body output type consists of the variables that the loop modifies
(in this case the counter `i` and the `value`)
plus a special boolean output called `should_continue` that tells the loop when to finish.

In our example the loop body increments the counter `i`,
adds `step` to the `value`
and checks if `i` has met the bound.

```{code-cell} ipython
from tierkreis.builtins.stubs import iadd, igt, rand_int

i = loop_body.task(iadd(loop_body.const(1), loop_body.inputs.i))
value = loop_body.task(iadd(loop_body.inputs.step, loop_body.inputs.value))
pred = loop_body.task(igt(loop_body.inputs.bound, i))
loop_body.outputs(LoopBodyOutput(i=i, value=value, should_continue=pred))
```

The main graph constructs the initial values for the loop and uses `GraphBuilder.loop` to run the loop.

```{code-cell} ipython3
from tierkreis.models import EmptyModel

f = GraphBuilder(EmptyModel, TKR[int])
init = LoopBodyInput(f.const(0), f.const(0), f.const(2), f.const(10))
loop_output = f.loop(loop_body, init)
f.outputs(loop_output.value)
```

### Tracking Outputs

We can also track intermediate values in a loop.
This is useful for tracking the intermediate values for validation or visualization.
For example, we could track the loss of a training loop.
We simple provide an additional name to the loop.

```{code-cell} ipython3
from tierkreis.models import EmptyModel

f = GraphBuilder(EmptyModel, TKR[int])
init = LoopBodyInput(f.const(0), f.const(0), f.const(2), f.const(10))
loop_output = f.loop(loop_body, init, "my_loop")
f.outputs(loop_output.value)
```

We can refer to this name after execution.

### Repeat until success

In addition to bounded iteration, the `GraphBuilder.loop` method can also define a 'repeat until success' loop.
First we need to create the graph that constitutes the body of the loop.
As a toy example, we choose a random number between 1 and 10 inclusive and continue if it is less than 10.

```{code-cell} ipython3
from typing import NamedTuple

from tierkreis.builder import GraphBuilder
from tierkreis.builtins.stubs import iadd, igt, rand_int
from tierkreis.models import TKR


class LoopBodyInput(NamedTuple):
    i: TKR[int]


class LoopBodyOutput(NamedTuple):
    i: TKR[int]
    should_continue: TKR[bool]


body = GraphBuilder(LoopBodyInput, LoopBodyOutput)
i = body.task(iadd(body.const(1), body.inputs.i))
a = body.task(rand_int(body.const(0), body.const(10)))
pred = body.task(igt(body.const(10), a))
body.outputs(LoopBodyOutput(i=i, should_continue=pred))
```

The main graph runs the loop and tells us the iteration on which we found success.

```{code-cell} ipython3
from tierkreis.models import EmptyModel

g = GraphBuilder(EmptyModel, TKR[int])
loop_output = g.loop(body, LoopBodyInput(g.const(0)))
g.outputs(loop_output.i)
```

# Execution

Since we still only use built-in functions, we execute the graph in the same way as before.

```{code-cell} ipython3
from uuid import UUID
from pathlib import Path

from tierkreis import run_graph
from tierkreis.storage import FileStorage, read_outputs, read_loop_trace
from tierkreis.executor import ShellExecutor

storage = FileStorage(UUID(int=99), name="Nested graphs using Eval")
executor = ShellExecutor(Path("."), workflow_dir=storage.workflow_dir)

storage.clean_graph_files()
run_graph(storage, executor, f.get_data(), {})
print(read_outputs(f, storage))
print(read_loop_trace(f, storage, "my_loop"))

storage.clean_graph_files()
run_graph(storage, executor, g.get_data(), {})
print(read_outputs(g, storage))
```
