# Tierkreis documentation

Tierkreis `[ˈtiːɐ̯ˌkʁaɪ̯s]` is an HPC native workflow management system (WMS) for asynchronous hybrid jobs.
Tierkreis combines task-based workers with context dependent execution on various storage implementations.

| WMS       | Multi-Language | Multi-Executor | HPC-Support | Visualization | Repeat until convergence | Proprietary Workers |
| --------- | :------------: | :------------: | :---------: | :-----------: | :----------------------: | :-----------------: |
| tierkreis |       ✔        |       ✔        |      ✔      |       ✔       |            ✔             |          ✔          |

## Introduction

Tierkreis allows users to define hybrid workflows as computational graphs
```python
from typing import NamedTuple

from tierkreis.builder import Graph
from tierkreis.builtins import iadd
from tierkreis.models import TKR
from tierkreis.runtime import new_default


class InParams(NamedTuple):
    a: TKR[int]
    b: TKR[int]
    c: TKR[int]


g = Graph(InParams, TKR[int])
x = g.task(iadd(g.inputs.a, g.inputs.b))
y = g.task(iadd(x, g.inputs.c))
workflow = g.finish_with_outputs(y)
```
and run them on different execution platforms with the Rust runtime:

```python
inputs = InParams(1, 2, 3)._asdict()

runtime = await new_default()
with runtime:
    workflow_id = await runtime.save_workflow("Hello World Graph", workflow)
    run_id = await runtime.start_new_run(workflow_id, inputs)
    await runtime.wait_for(run_id)
    result = await runtime.get_outputs(run_id)
```

## Quick Start

To get up to speed with tierkreis we recommend the [Beginners Tutorial](./tutorial/index.md).
It will guide you through the core concepts of Tierkreis, how to construct graphs, define workers, and execute them.
For more advanced topics see the specific contents below.

## Contents

```{toctree}
:maxdepth: 3
installation.md
tutorial/index
```

```{toctree}
:maxdepth: 2
:caption: User guide
tutorial/core_concepts.md
tutorial/visualization.md
tutorial/logging_and_errors.md
tutorial/cli
graphs/index
worker/index
executors/index
tutorial/tutorials.md
```

```{toctree}
:maxdepth: 2
:caption: API Reference
apidocs/index
```

## Additional Material

A small demo tutorial presented at QCUF 2026 can be found in [this][https://github.com/Quantinuum/tierkreis-tutorial] repository.
