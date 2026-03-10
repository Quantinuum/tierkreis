---
file_format: mystnb
kernelspec:
  name: python3
---

# Parallel computation using Map

An example of how to run several CPU intensive nodes in parallel and aggregate the results.

## Example worker

We use an encryption function as a placeholder for a CPU intensive worker function.
For demonstration purposes we ensure that the plaintexts are short enough (and the work factor small enough) that the graph runs in under 10s.

The following worker is in the [Tierkreis GitHub repo](https://github.com/Quantinuum/tierkreis) at `docs/source/examples/example_workers/auth_worker`:

```{literalinclude} ../examples/example_workers/auth_worker/tkr_auth_worker_impl/impl.py
:language: python
```

## Generating stubs

Since this worker uses the Tierkreis Python library, we can automatically generate stub files using the following command.
The stub files will provide us with type hints in the graph building process later on.

```{code-cell}
!cd ../examples/example_workers/auth_worker && uv run tkr_auth_worker_impl/main.py --stubs-path ./api/api.py > /dev/null 2>&1
```

## Writing a graph

We can import this stub file to help create our graph.

The graph builder manipulates references to values, not the values themselves.
(The one exception to this rule is when we add a constant value to a graph using `GraphBuilder.const`. Then the actual value is added to the graph definition and `GraphBuilder.const` returns a reference to this value.)
The references are type checked using the `TKR` type.
I.e. a reference to an `int` has the type `TKR[int]`.

```{code-cell}
from typing import NamedTuple
from tierkreis.models import EmptyModel, TKR
from tierkreis.builder import GraphBuilder
from tierkreis.builtins import mean

from auth_worker import encrypt, EncryptionResult


def map_body():
    g = GraphBuilder(TKR[str], EncryptionResult)
    result = g.task(encrypt(plaintext=g.inputs, work_factor=g.const(2**14)))
    g.outputs(result)
    return g


class GraphOutputs(NamedTuple):
    average_time_taken: TKR[float]
    ciphertexts: TKR[list[str]]


def graph():
    g = GraphBuilder(EmptyModel, GraphOutputs)
    plaintexts = g.const([f"plaintext+{n}" for n in range(20)])
    results = g.map(map_body(), plaintexts)

    ciphertexts = g.map(lambda x: x.ciphertext, results)
    times = g.map(lambda x: x.time_taken, results)

    av = g.task(mean(values=times))
    out = GraphOutputs(ciphertexts=ciphertexts, average_time_taken=av)

    g.outputs(out)
    return g
```

## Running the graph

In order to run a graph we need to choose a storage backend and executor.
In this example we choose a simple filestorage backend and the UV executor.
For the UV executor the registry path should be a folder containing all the workers we use.

Then we pass the storage, executor and graph into the `run_graph` function.
At this point we have the option to pass additional graph inputs.

```{code-cell}

import json
from pathlib import Path
import time
from uuid import UUID
from tierkreis import run_graph
from tierkreis.executor import UvExecutor
from tierkreis.storage import FileStorage, read_outputs

storage = FileStorage(UUID(int=2048), "auth_graph", do_cleanup=True)
executor = UvExecutor(
    registry_path=Path("../examples/example_workers"), logs_path=storage.logs_path
)
start = time.time()
run_graph(storage, executor, graph().data, {})
total_time = time.time() - start

outputs = read_outputs(graph(), storage)
av = outputs["average_time_taken"]
ciphertexts = outputs["ciphertexts"]
print(f"Encrypted 20 plaintexts in {total_time:1g}s with mean encryption time {av:1g}")
```

We should see that the mean time to encrypt a single plaintext is quite close to the time taken for the whole workflow, which indicates that the encryptions were run in parallel.
