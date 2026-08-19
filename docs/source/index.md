# Tierkreis documentation

Tierkreis `[ˈtiːɐ̯ˌkʁaɪ̯s]` is an HPC native workflow management system (WMS) for asynchronous hybrid jobs.
Tierkreis combines task-based workers with context dependent execution on various storage implementations.

| WMS       | Multi-Language | Multi-Executor | HPC-Support | Visualization | Repeat until convergence | Proprietary Workers |
| --------- | :------------: | :------------: | :---------: | :-----------: | :----------------------: | :-----------------: |
| tierkreis |       ✔        |       ✔        |      ✔      |       ✔       |            ✔             |          ✔          |

## Introduction

Tierkreis allows users to define hybrid workflows as computational graphs
```python
class InParams(NamedTuple):
    a: TKR[float]
    b: TKR[float]
    c: TKR[float]


g = Graph(InParams, TKR[float])
x = g.task(add(g.inputs.a, g.inputs.b))
y = g.task(add(x, g.inputs.c))
workflow = g.finish_with_outputs(y)
```
and run them on different execution platforms

```python
storage = FileStorage(workflow_id=UUID(int=12345), name="Hello World Graph")
executor = ShellExecutor(registry_path=None, workflow_dir=storage.workflow_dir)
inputs = InParams(0, 0.25, 0.5)._asdict()
run_graph(storage, executor, workflow, inputs)
result = read_outputs(workflow, storage)
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
