from typing import Literal


def python_worker_main(worker_name: str) -> str:
    worker_name = worker_name.replace("-", "_")
    return f"""from sys import argv

from tierkreis import Worker
from tierkreis.exceptions import TierkreisError

worker = Worker("{worker_name}")

@worker.task()
def your_worker_task(value: int) -> int:
    return value


def main():
    worker.app(argv)


if __name__ == "__main__":
    main()

"""


def python_worker_workspace_pyproject(worker_name: str, external: bool = False) -> str:
    worker_name = worker_name.replace("_", "-")
    template = f"""[project]
name = "tkr-{worker_name}"
version = "0.1.0"
description = "A tierkreis worker."
readme = "README.md"
requires-python = ">=3.12"
authors = [ {{name = "Your Name", email = "you@example.com"}} ]
dependencies = [
    "tierkreis",
]
[project.optional-dependencies]
"""
    if not external:
        template += f"""src = [
    "tkr-{worker_name}-src",
]
"""
    template += f"""api = [
    "tkr-{worker_name}-api",
]

[tool.uv.sources]
"""
    if not external:
        template += f"tkr-{worker_name}-src = {{ workspace = true }}\n"
    template += f"""tkr-{worker_name}-api = {{ workspace = true }}

[tool.uv.workspace]
members = [{'\n"src",\n' if not external else ""}
    "api",
]
"""
    return template


def python_worker_pyproject(
    worker_name: str, kind: Literal["api", "src"] = "api"
) -> str:
    worker_name = worker_name.replace("_", "-")
    template = f"""[project]
name = "tkr-{worker_name}-{kind}"
version = "0.1.0"
description = "A tierkreis worker implementation."
readme = "README.md"
requires-python = ">=3.12"
authors = [ {{name = "Your Name", email = "you@example.com"}} ]
dependencies = [
    "tierkreis",
]

[build-system]
requires = ["hatchling"]
build-backend = "hatchling.build"

"""
    if kind == "src":
        template += f"""[project.scripts]
tkr_{worker_name} = "main:main"

"""
    return template


def external_worker_idl(worker_name: str) -> str:
    return f"""model YourModel {{
        value: int
}}
    
interface {worker_name} {{
    your_function(value: int): YourModel;
}}

"""


def default_graph(worker_name: str) -> str:
    worker_name = worker_name.replace("-", "_")
    return f"""from typing import NamedTuple
from pathlib import Path
from uuid import UUID
    
from tierkreis.builder import GraphBuilder
from tierkreis.controller import run_graph
from tierkreis.controller.data.models import TKR, OpaqueType
from tierkreis.executor import UvExecutor
from tierkreis.storage import FileStorage, read_outputs

from tkr.workers.{worker_name}.api.api import your_worker_task

class GraphInputs(NamedTuple):
    value: TKR[int]


class GraphOutputs(NamedTuple):
    value: TKR[int]

    
def your_graph() -> GraphBuilder[GraphInputs, GraphOutputs]:
    g = GraphBuilder(GraphInputs, GraphOutputs)
    out = g.task(your_worker_task(g.inputs.value))
    g.outputs(GraphOutputs(value=out))
    return g
    
def main() -> None:
    graph = your_graph()
    storage = FileStorage(workflow_id=UUID(int=12345), name="your_graph")
    executor = UvExecutor(
        Path(__file__).parent.parent / "workers", storage.logs_path
    )
    storage.clean_graph_files()
    run_graph(storage, executor, graph.get_data(), {{"value": 1}})
    result = read_outputs(graph, storage)
    print("Value is: ", result)

if __name__ == "__main__":
    main()

"""
