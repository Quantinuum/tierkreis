# Tierkreis

Tierkreis is a higher-order dataflow graph representation and runtime designed
for compositional quantum-classical hybrid algorithms. The Python package
provides graph construction, worker development, and bindings to the Rust
runtime.

For background, see [Tierkreis: a Dataflow Framework for Hybrid
Quantum-Classical Computing](https://arxiv.org/abs/2211.02350).

## Quick start

Tierkreis works best with the [uv package manager][uv]:

```bash
uv init
uv add tierkreis
uv run tkr init project
uv run tkr run -o
```

`tkr run` loads `./tkr/graphs/main.py:workflow` and
`workflow_inputs.json` by default. Use `-g module:function` for another Python
graph, `-f` for a serialized graph, `-i` for input files, and `--config` for a
runtime TOML file.

## Running a graph from Python

```python
from typing import NamedTuple

from tierkreis.builder import Graph
from tierkreis.builtins import itimes
from tierkreis.models import TKR
from tierkreis.runtime import new_default


class Inputs(NamedTuple):
    value: TKR[int]


graph = Graph(Inputs, TKR[int])
workflow = graph.finish_with_outputs(
    graph.task(itimes(graph.inputs.value, graph.const(5)))
)

runtime = await new_default()
with runtime:
    workflow_id = await runtime.save_workflow("times five", workflow)
    run_id = await runtime.start_new_run(workflow_id, {"value": 3})
    await runtime.wait_for(run_id)
    output = await runtime.get_outputs(run_id)
```

The runtime assigns UUIDs to saved workflows and runs. Attempt zero is created
by `start_new_run`; `restart_task` can later create another attempt while
retaining unaffected results and preserving the source attempt.

## Runtime architecture

- **Runtime state** stores workflows, runs, attempts, and node status. Memory
  and SQLite implementations are available.
- **Asset storage** transfers serialized graph inputs and task outputs. Memory
  and file-backed implementations are available.
- **Executors** dispatch runnable tasks. The runtime supports in-memory
  builtins, installed subprocess workers, Nexus, QRMI, and PBS/PJSUB/Slurm.
- **Workers** are standalone programs implementing typed task namespaces.

State, asset stores, executors, resource profiles, and worker commands are
selected in `tierkreis.toml`. Nested Eval, Loop, Map, and conditional graphs
load their inputs lazily. A persistent SQLite runtime can recover active runs
after the runtime process is restarted.

## Documentation

See the [full documentation][docs-home] and [getting-started guide][docs-getting-started]
for graph construction, workers, configuration, debugging, and HPC execution.

[docs-home]: https://quantinuum.github.io/tierkreis/
[docs-getting-started]: https://quantinuum.github.io/tierkreis/tutorial/index.html
[uv]: https://docs.astral.sh/uv/
