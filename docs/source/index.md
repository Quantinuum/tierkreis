# Tierkreis documentation

Tierkreis `[ˈtiːɐ̯ˌkʁaɪ̯s]` is an HPC native workflow management system (WMS) for asynchronous hybrid jobs.
Tierkreis combines task-based workers with context dependent execution on various storage implementations.

| WMS       | Multi-Language | Multi-Executor | HPC-Support | Visualization | Repeat until convergence | Proprietary Workers |
| --------- | :------------: | :------------: | :---------: | :-----------: | :----------------------: | :-----------------: |
| tierkreis |       ✔        |       ✔        |      ✔      |       ✔       |            ✔             |          ✔          |

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
