"""The workflow controller for Tierkreis.

This is the main functionality controlling the execution of Tierkreis graphs.
It provides the main entry point for running a graph,
 and the main loop for resuming a graph until completion.
"""

import logging
from time import sleep
from typing import TYPE_CHECKING

from tierkreis.controller.data.graph import Eval, GraphData
from tierkreis.controller.data.location import Loc
from tierkreis.controller.data.models import TModel
from tierkreis.controller.data.types import PType, bytes_from_ptype, ptype_from_bytes
from tierkreis.controller.executor.protocol import ControllerExecutor
from tierkreis.controller.start import start, start_nodes
from tierkreis.controller.storage.protocol import ControllerStorage
from tierkreis.controller.storage.walk import walk_node
from tierkreis.controller.storage.walk_result import NodeRunData
from tierkreis.exceptions import TierkreisError
from tierkreis.logger_setup import set_tkr_logger
from tierkreis.models import Workflow

if TYPE_CHECKING:
    from tierkreis.controller.data.core import PortID, ValueRef

root_loc = Loc("")
logger = logging.getLogger(__name__)


def run_graph[A: TModel, B: TModel](
    storage: ControllerStorage,
    executor: ControllerExecutor,
    g: GraphData | Workflow[A, B],
    graph_inputs: dict[str, PType] | PType,
    n_iterations: int = 8640000,
    polling_interval_seconds: float = 0.01,
    *,
    enable_logging: bool = True,
    enable_breakpoints: bool = False,
) -> None:
    """Start a graph execution.

    Kicks of the execution by writing the graph inputs to storage.
    Also marks the top-level eval wrapping the graph asa ready.

    :param storage: The storage backend for the controller.
    :type storage: ControllerStorage
    :param executor: The executor backend for the controller.
    :type executor: ControllerExecutor
    :param g: The graph to run.
    :type g: GraphData | Workflow[A, B]
    :param graph_inputs: The inputs to the graph.
     If a single PType is provided, it will be provided as the input "value".
    :type graph_inputs: dict[str, PType] | PType
    :param n_iterations: The maximum number of iterations to run the graph,
        defaults to 8640000
    :type n_iterations: int, optional
    :param polling_interval_seconds: The polling interval in seconds, defaults to 0.01
    :type polling_interval_seconds: float, optional
    :param enable_logging: Whether to enable logging, defaults to True
    :type enable_logging: bool, optional
    :param enable_breakpoints: Whether to enable breakpoint nodes, defaults to False
    :type enable_breakpoints: bool, optional
    :raises TierkreisError: If the graph encounters errors during execution.
    """
    if isinstance(g, Workflow):
        g = g.data

    if not isinstance(graph_inputs, dict):
        graph_inputs = {"value": graph_inputs}
    remaining_inputs = g.remaining_inputs(set(graph_inputs.keys()))
    if len(remaining_inputs) > 0:
        logger.warning(
            "Some inputs were not provided: %s"
            "Tasks will use default values if available.",
            remaining_inputs,
        )

    storage.write_metadata(Loc(""))
    if enable_logging:
        set_tkr_logger(storage.logs_path)

    for name, value in graph_inputs.items():
        storage.write_output(root_loc.N(-1), name, bytes_from_ptype(value))

    storage.write_output(root_loc.N(-1), "body", bytes_from_ptype(g))

    inputs: dict[PortID, ValueRef] = {
        k: (-1, k) for k, _ in graph_inputs.items() if k != "body"
    }
    node_run_data = NodeRunData(
        node_location=Loc(), node=Eval((-1, "body"), inputs), output_list=[]
    )
    start(storage, executor, node_run_data)
    resume_graph(
        storage,
        executor,
        n_iterations,
        polling_interval_seconds,
        enable_breakpoints=enable_breakpoints,
    )


def resume_graph(
    storage: ControllerStorage,
    executor: ControllerExecutor,
    n_iterations: int = 8640000,
    polling_interval_seconds: float = 0.01,
    *,
    enable_breakpoints: bool = False,
) -> None:
    """Resume a graph after initial start.

    This iteratively walks the graph to find new nodes to start.
    A node is ready to start once all its inputs are available.
    Starts from constructing the dependencies by starting from the output node
     and walking backwards.

    :param storage: The storage backend for the controller.
    :type storage: ControllerStorage
    :param executor: The executor backend for the controller.
    :type executor: ControllerExecutor
    :param n_iterations: The maximum number of iterations to run the graph,
        defaults to 8640000
    :type n_iterations: int, optional
    :param polling_interval_seconds: The polling interval in seconds, defaults to 0.01
    :type polling_interval_seconds: float, optional
    :param enable_breakpoints: Whether to enable breakpoint nodes, defaults to False
    :type enable_breakpoints: bool, optional
    :raises TierkreisError: If the graph encounters errors during execution.
    """
    message = storage.read_output(Loc().N(-1), "body")
    graph = ptype_from_bytes(message, GraphData)

    for _ in range(n_iterations):
        walk_results = None
        if enable_breakpoints:
            walk_results = storage.read_breakpoints()
        if walk_results is None:
            walk_results = walk_node(storage, Loc(), graph.output_idx(), graph)
        if walk_results.errored != []:
            storage.touch(storage._error_path(Loc()))
            node_errors = "\n".join(x for x in walk_results.errored)
            storage.write_node_errors(Loc(), node_errors)

            logger.error("\n\nGraph finished with errors.\n\n")
            for error_loc in walk_results.errored:
                logger.error(storage.read_errors(error_loc))
                logger.error("Node: '%s' encountered an error.", error_loc)
                logger.error(
                    "Stderr information is available at %s",
                    storage._worker_logs_path(error_loc),
                )

            logger.error("--- Tierkreis graph errors above this line. ---")
            msg = "Graph encountered errors"
            raise TierkreisError(msg)

        if enable_breakpoints and (walk_results.breakpoints != []):
            storage.write_breakpoint(walk_results)
            break

        start_nodes(storage, executor, walk_results.inputs_ready)
        if storage.is_node_finished(Loc()):
            storage.write_workflow_completion_time()
            break
        sleep(polling_interval_seconds)
