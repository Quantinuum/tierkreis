# Tierkreis documentation

Tierkreis `[ˈtiːɐ̯ˌkʁaɪ̯s]` is an HPC native workflow management system (WMS) for asynchronous hybrid jobs.
Tierkreis combines task-based workers with context dependent execution on various storage implementations.

See Tierkreis in action:

<video autoplay muted loop style="max-width: 100%;">
  <source src="../_static/demo_wf.mp4" type="video/mp4">
</video>

## Motivation & Goal

Most useful quantum computing algorithms combine classical and quantum tasks.
Examples include, QSCI, QM/MM methods, QMCMC and other algorithms.
Therefore, they operate in a __hybrid__ paradigm which connect classical and quantum resources.
Implementing hybrid algorithms proposes multiple challenges.
1. Heterogeneous Hardware: A multitude of hardware has to be supported. 
This includes HPC hardware with unique architectures, GPU-acellerated systems, and quantum computers.
2. Proprietary and existing software: Many algorithms use highly optimized preexisting frameworks or custom software. 
They can be written in any language and bring their own dependencies.
3. Supportive functions: Repeadability, monitoring and debugging become increasingly complex with an increased number of systems.

**Tierkreis** aims to simplify the development, deployment and scaling of hybrid workflows.
Some of its core features are listed below:

| WMS       | Multi-Language | Hardware Profiles | HPC-Support | Visualization | Monitoring and Reporting | Proprietary Workers |
| --------- | :------------: | :---------------: | :---------: | :-----------: | :----------------------: | :-----------------: |
| tierkreis |       ✔        |       ✔           |      ✔      |       ✔       |            ✔             |          ✔          |

1. Multi-Language: Workers in any programming language are supported with a lightweight wrapper.
2. Hardware Profiles: Run the same workflow locally, in HPC centers or in the cloud, by simply switching the harwdware profiles.
3. HPC-Support: Built-in support for HPC workload managers like SLURM, PBS and PJSub.
4. Visualization: See the graph you're building and how it executes in real time.
5. Errors and Reporting: Localized errors and debugging tools help narrow down issues. Detailed logging and OTEL provide system information.
Automatic checkpoints help recover after errors without recomputing previous tasks.
6. Proprietary Workers: By exposing the API users can use your software without access to the source. Third-party dependencies are minimized.

## Installation

Tierkreis consist of multiple packages:

- `tierkreis`: the core functionality including the
- independent [workers](./tutorial/core_concepts) consisting of API and implementation packages
    - `tkr-*-worker`: **only the API**
    - `tkr-*-worker-impl`: API + Implementation 

The libraries can be installed with `pip` but we recommend using [uv](https://docs.astral.sh/uv/) read more in the [Getting Started](./tutorial/index.md) docs.

```
uv add tierkreis
uv add tkr-*-worker
```



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
and run them on different execution platforms:

```python
inputs = InParams(1, 2, 3)._asdict()

runtime = await new_default()
with runtime:
    workflow_id = await runtime.save_workflow("Hello World Graph", workflow)
    run_id = await runtime.start_new_run(
        workflow_id, inputs
    )  # optional profile, e.g. profile="hpc"
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

- A small demo tutorial presented at QCUF 2026 can be found in [this](https://github.com/Quantinuum/tierkreis-tutorial) repository.
- An implementation of multiple workflows in the contoext of the openQSE SC 26 Demo can be found [here](https://github.com/Quantinuum/tierkreis-sc-demo)
