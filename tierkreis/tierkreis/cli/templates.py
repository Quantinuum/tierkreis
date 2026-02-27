"""String template for the project initialization."""


def python_worker_main(worker_name: str) -> str:
    """Generate a python morker main.py.

    :param worker_name: The name of the worker.
    :type worker_name: str
    :return: The generated main.py content.
    :rtype: str
    """
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


def python_worker_pyproject(worker_name: str) -> str:
    """Generate the pyproject.toml for the worker workspace.

    :param worker_name: Name of the worker.
    :type worker_name: str
    :return: The generated pyproject.toml content.
    :rtype: str
    """
    worker_name = worker_name.replace("_", "-")
    template = f"""[project]
name = "tkr-{worker_name}"
version = "0.1.0"
description = "A tierkreis worker implementation."
readme = "README.md"
requires-python = ">=3.12"
authors = [ {{name = "Your Name", email = "you@example.com"}} ]
dependencies = [
    "tierkreis",
]

[tool.uv.workspace]
members = [
    "src/api",
]

[tool.uv.sources]
tkr-{worker_name}-api = {{ workspace = true }}

[build-system]
requires = ["hatchling"]
build-backend = "hatchling.build"

[tool.hatch.build.targets.wheel]
packages = ["src"]

[project.scripts]
tkr_{worker_name} = "src.main:main"

"""
    return template


def python_worker_api_pyproject(worker_name: str) -> str:
    """Generate the pyproject.toml for the worker.

    Either for the api directory (only stubs) used during build time or
    the src directory (the actual worker implementation) used during runtime.

    :param worker_name: Name of the worker.
    :type worker_name: str
    :rtype: str
    """
    package_name = worker_name.replace("_", "-")
    template = f"""[project]
name = "tkr-{package_name}-api"
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

[tool.hatch.build.targets.wheel.force-include]
"api.py" = "{worker_name}.py"

"""
    return template


def external_worker_idl(worker_name: str) -> str:
    """Generate a typespec file for a worker.

    :param worker_name: The name of the worker.
    :type worker_name: str
    :return: The generated typespec content.
    :rtype: str
    """
    return f"""model YourModel {{
        value: int
}}

interface {worker_name} {{
    your_function(value: int): YourModel;
}}

"""


def default_graph(worker_name: str) -> str:
    """Generate a default graph example using a worker.

    :param worker_name: The name of the worker.
    :type worker_name: str
    :return: The generated main graph content.
    :rtype: str
    """
    worker_name = worker_name.replace("-", "_")
    return f"""from typing import NamedTuple
from pathlib import Path
from uuid import UUID

from tierkreis.builder import GraphBuilder
from tierkreis.controller import run_graph
from tierkreis.controller.data.models import TKR, OpaqueType
from tierkreis.executor import ShellExecutor, UvExecutor
from tierkreis.storage import FileStorage, read_outputs

from {worker_name} import your_worker_task

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
    # Use the following executor to run installed workers like tkr-*-worker
    # executor = ShellExecutor(Path(), storage.workflow_dir)
    # To use both look at the following:
    # https://quantinuum.github.io/tierkreis/executors/index.html#combining-executors
    storage.clean_graph_files()
    run_graph(storage, executor, graph.get_data(), {{"value": 1}})
    result = read_outputs(graph, storage)
    print("Value is: ", result)

if __name__ == "__main__":
    main()

"""


def worker_init() -> str:
    """Generate a toplevel init for the worker.

    :return: An init template
    :rtype: str
    """
    return """
from .src.api.api import *

"""
