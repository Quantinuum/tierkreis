# /// script
# requires-python = ">=3.12"
# dependencies = ["tierkreis"]
#
# [tool.uv.sources]
# tierkreis = { path = "../tierkreis", editable = true }
# ///
from pathlib import Path
from uuid import UUID

from tierkreis.builder import GraphBuilder
from tierkreis.controller import run_graph
from tierkreis.controller.data.core import EmptyModel
from tierkreis.controller.data.location import Loc
from tierkreis.controller.data.models import TKR
from tierkreis.controller.storage.filestorage import ControllerFileStorage
from tierkreis.controller.executor.uv_executor import UvExecutor
from tierkreis.exceptions import TierkreisError

from example_workers.error_worker.stubs import fail

root_loc = Loc()


def error_graph() -> GraphBuilder:
    """A graph that errors."""

    g = GraphBuilder(EmptyModel, TKR[str])
    output = g.task(fail())
    g.outputs(output)
    return g


def main() -> None:
    """Configure our workflow execution and run it to completion."""
    # Assign a fixed uuid for our workflow.
    workflow_id = UUID(int=103)
    storage = ControllerFileStorage(workflow_id, name="error_handling", do_cleanup=True)

    # Look for workers in the `example_workers` directory.
    registry_path = Path(__file__).parent / "example_workers"
    executor = UvExecutor(registry_path=registry_path, logs_path=storage.logs_path)
    print("Starting workflow at location:", storage.logs_path)
    try:
        run_graph(
            storage,
            executor,
            error_graph().data,
            {"value": "world!"},
            polling_interval_seconds=0.1,
        )
    except TierkreisError:
        output = storage.read_errors(root_loc)
        print("Errors are at:", output)


if __name__ == "__main__":
    main()
