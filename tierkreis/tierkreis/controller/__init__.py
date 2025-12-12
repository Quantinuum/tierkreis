import logging
from pathlib import Path
from time import sleep
from uuid import UUID

from tierkreis.builder import GraphBuilder
from tierkreis.controller.data.graph import Eval, GraphData
from tierkreis.controller.data.location import Loc
from tierkreis.controller.data.models import PModel
from tierkreis.controller.data.types import PType, bytes_from_ptype, ptype_from_bytes
from tierkreis.controller.executor.in_memory_executor import InMemoryExecutor
from tierkreis.controller.executor.protocol import ControllerExecutor
from tierkreis.controller.start import NodeRunData, start, start_nodes
from tierkreis.controller.storage.in_memory import ControllerInMemoryStorage
from tierkreis.controller.storage.protocol import ControllerStorage
from tierkreis.controller.storage.walk import walk_node
from tierkreis.controller.data.core import PortID, ValueRef

root_loc = Loc("")
logger = logging.getLogger(__name__)


def run_graph(
    storage: ControllerStorage,
    executor: ControllerExecutor,
    g: GraphData | GraphBuilder,
    graph_inputs: dict[str, PType] | PType,
    n_iterations: int = 10000,
    polling_interval_seconds: float = 0.01,
) -> None:
    if isinstance(g, GraphBuilder):
        g = g.get_data()

    if not isinstance(graph_inputs, dict):
        graph_inputs = {"value": graph_inputs}
    remaining_inputs = g.remaining_inputs({k for k in graph_inputs.keys()})
    if len(remaining_inputs) > 0:
        logger.warning(f"Some inputs were not provided: {remaining_inputs}")

    storage.write_metadata(Loc(""))
    for name, value in graph_inputs.items():
        storage.write_output(root_loc.N(-1), name, bytes_from_ptype(value))

    storage.write_output(root_loc.N(-1), "body", bytes_from_ptype(g))

    inputs: dict[PortID, ValueRef] = {
        k: (-1, k) for k, _ in graph_inputs.items() if k != "body"
    }
    node_run_data = NodeRunData(Loc(), Eval((-1, "body"), inputs), [])
    start(storage, executor, node_run_data)
    resume_graph(storage, executor, n_iterations, polling_interval_seconds)


def debug_graph(
    g: GraphData | GraphBuilder,
    graph_inputs: dict[str, PType] | PType,
    registry_path: Path,
    debug_values: dict[type, PModel] | None = None,
    use_debug_values: bool = True,
    n_iterations: int = 10000,
    polling_interval_seconds: float = 0.01,
) -> ControllerInMemoryStorage:
    storage = ControllerInMemoryStorage(UUID(int=0), "debug_graph")
    if debug_values is not None:
        dbg_values = {
            f"{k().namespace}:{k.__name__}": v for k, v in debug_values.items()
        }
    else:
        dbg_values = None
    executor = InMemoryExecutor(registry_path, storage, dbg_values)
    # We could invoke run_graph but I prefer to keep `debug` outside of the interface

    if isinstance(g, GraphBuilder):
        g = g.get_data()
    if not isinstance(graph_inputs, dict):
        graph_inputs = {"value": graph_inputs}
    remaining_inputs = g.remaining_inputs({k for k in graph_inputs.keys()})
    if len(remaining_inputs) > 0:
        logger.warning(f"Some inputs were not provided: {remaining_inputs}")

    storage.write_metadata(Loc(""))
    for name, value in graph_inputs.items():
        storage.write_output(root_loc.N(-1), name, bytes_from_ptype(value))

    storage.write_output(root_loc.N(-1), "body", bytes_from_ptype(g))

    inputs: dict[PortID, ValueRef] = {
        k: (-1, k) for k, _ in graph_inputs.items() if k != "body"
    }
    node_run_data = NodeRunData(Loc(), Eval((-1, "body"), inputs), [])
    start(storage, executor, node_run_data, debug=use_debug_values)
    resume_graph(
        storage,
        executor,
        n_iterations,
        polling_interval_seconds,
        debug=use_debug_values,
    )
    return storage


def resume_graph(
    storage: ControllerStorage,
    executor: ControllerExecutor,
    n_iterations: int = 10000,
    polling_interval_seconds: float = 0.01,
    debug: bool = False,
) -> None:
    message = storage.read_output(Loc().N(-1), "body")
    graph = ptype_from_bytes(message, GraphData)

    for _ in range(n_iterations):
        walk_results = walk_node(storage, Loc(), graph.output_idx(), graph)
        if walk_results.errored != []:
            # TODO: add to base class after storage refactor
            (storage.logs_path.parent / "-" / "_error").touch()
            node_errors = "\n".join(x for x in walk_results.errored)
            storage.write_node_errors(Loc(), node_errors)

            print("\n\nGraph finished with errors.\n\n")

            for error_loc in walk_results.errored:
                print(error_loc)
                print(storage.read_errors(error_loc))
                print("\n\n")

            print("--- Tierkreis graph errors above this line. ---\n\n")
            break

        start_nodes(storage, executor, walk_results.inputs_ready, debug)
        if storage.is_node_finished(Loc()):
            break
        sleep(polling_interval_seconds)
