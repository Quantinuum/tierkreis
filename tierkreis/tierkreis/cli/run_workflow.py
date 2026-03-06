"""Implementation to run a workflow."""

import logging
import uuid
from pathlib import Path

from tierkreis.controller import run_graph
from tierkreis.controller.data.graph import GraphData
from tierkreis.controller.data.types import PType
from tierkreis.controller.executor.shell_executor import ShellExecutor
from tierkreis.controller.executor.uv_executor import UvExecutor
from tierkreis.controller.storage.filestorage import ControllerFileStorage
from tierkreis.storage import read_outputs

logger = logging.getLogger(__name__)


def run_workflow(  # noqa: PLR0913
    graph: GraphData,
    inputs: dict[str, PType],
    name: str | None = None,
    run_id: int | None = None,
    log_level: int | str = logging.INFO,
    registry_path: Path | None = None,
    n_iterations: int = 10000,
    polling_interval_seconds: float = 0.1,
    *,
    print_output: bool = False,
    use_uv_executor: bool = False,
) -> None:
    """Run a workflow.

    Wrapper for :py:func:`tierkreis.controller.run_graph` to run a workflow.
    Adds some sensible defaults.

    :param graph: The graph to run.
    :type graph: GraphData
    :param inputs: The inputs to the workflow.
    :type inputs: dict[str, PType]
    :param name: The name of the workflow, defaults to None
    :type name: str | None, optional
    :param run_id: The run ID of the workflow, defaults to None
    :type run_id: int | None, optional
    :param log_level: The log level for the workflow, defaults to logging.INFO
    :type log_level: int | str, optional
    :param registry_path: The worker registry, defaults to Path(__file__).parent
    :type registry_path: Path | None, optional
    :param print_output: Whether to print final outputs, defaults to False
    :type print_output: bool, optional
    :param use_uv_executor: Use the UV executor instead of ShellExecutor
        , defaults to False
    :type use_uv_executor: bool, optional
    :param n_iterations: The maximum number of iterations, defaults to 10000
    :type n_iterations: int, optional
    :param polling_interval_seconds: The controller tickrate, defaults to 0.1
    :type polling_interval_seconds: float, optional
    """
    logger.setLevel(log_level)
    workflow_id = uuid.uuid4() if run_id is None else uuid.UUID(int=run_id)
    logger.info("Workflow ID is %s", workflow_id)
    storage = ControllerFileStorage(workflow_id, name=name, do_cleanup=True)
    if use_uv_executor:
        if registry_path is None:
            registry_path = Path(__file__).parent
        executor = UvExecutor(registry_path=registry_path, logs_path=storage.logs_path)
    else:
        executor = ShellExecutor(registry_path, storage.workflow_dir)

    logger.info("Starting workflow at location: %s", storage.logs_path)

    run_graph(
        storage,
        executor,
        graph,
        inputs,
        n_iterations,
        polling_interval_seconds,
    )
    if print_output:
        all_outputs = read_outputs(graph, storage)
        if isinstance(all_outputs, dict):
            for output_name, output_value in all_outputs.items():
                print(f"'{output_name}': {output_value!r}")  # noqa: T201
        else:
            print(f"value: {all_outputs!r}")  # noqa: T201
