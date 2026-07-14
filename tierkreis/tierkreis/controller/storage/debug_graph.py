"""Debug wrapper for the main controller."""

from uuid import UUID

from tierkreis.consts import WORKERS_DIR
from tierkreis.controller import run_graph
from tierkreis.controller.data.graph import GraphData
from tierkreis.controller.data.models import TModel
from tierkreis.controller.data.types import PType, Workflow
from tierkreis.controller.executor.in_memory_executor import InMemoryExecutor
from tierkreis.controller.storage.in_memory import ControllerInMemoryStorage


def debug_graph[A: TModel, B: TModel](
    g: GraphData | Workflow[A, B],
    graph_inputs: dict[str, PType] | PType,
    n_iterations: int = 8640000,
    polling_interval_seconds: float = 0.01,
) -> ControllerInMemoryStorage:
    """Start a graph execution in debugging mode,.

    Uses debugging mode by setting up in memory storage and executor.
    This also means that only python code can be run.
    All workers must be available locally, too.

    :param g: The workflow to debug.
    :type g: GraphData | Workflow[A, B]
    :param graph_inputs: The inputs to the graph.
     If a single PType is provided, it will be provided as the input "value".
    :type graph_inputs: dict[str, PType] | PType
    :param n_iterations: The maximum number of iterations to run the graph,
        defaults to 8640000
    :type n_iterations: int, optional
    :param polling_interval_seconds: The polling interval in seconds, defaults to 0.01
    :type polling_interval_seconds: float, optional
    :return: The storage for reuse.
    :rtype: ControllerInMemoryStorage
    """
    storage = ControllerInMemoryStorage(UUID(int=0))
    executor = InMemoryExecutor(WORKERS_DIR, storage)
    run_graph(
        storage,
        executor,
        g,
        graph_inputs,
        n_iterations,
        polling_interval_seconds,
        enable_logging=True,
        enable_breakpoints=True,
    )
    return storage
