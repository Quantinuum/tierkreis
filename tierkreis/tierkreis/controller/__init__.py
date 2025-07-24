import logging
from time import sleep

from tierkreis.builder import GraphBuilder
from tierkreis.controller.data.location import Loc
from tierkreis.controller.data.types import PType, bytes_from_ptype, ptype_from_bytes
from tierkreis.controller.executor.protocol import ControllerExecutor
from tierkreis.controller.start import NodeRunData, start, start_nodes
from tierkreis.controller.storage.protocol import ControllerStorage
from tierkreis.controller.storage.walk import walk_node
from tierkreis_core import ExteriorRef, GraphData, new_eval_root, PortID

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

    storage.write_metadata(root_loc)
    for name, value in graph_inputs.items():
        storage.write_output(root_loc.exterior(), name, bytes_from_ptype(value))

    storage.write_output(root_loc.exterior(), "body", bytes_from_ptype(g))

    inputs: dict[PortID, ExteriorRef] = {
        k: ExteriorRef(k) for k in graph_inputs.keys() if k != "body"
    }
    graph_outputs = g.graph_outputs()
    if graph_outputs is None:
        raise ValueError("Cannot run a graph with no outputs.")

    node_run_data = NodeRunData(
        Loc(),
        new_eval_root(inputs),
        graph_outputs,
    )
    start(storage, executor, node_run_data)
    resume_graph(storage, executor, n_iterations, polling_interval_seconds)


def resume_graph(
    storage: ControllerStorage,
    executor: ControllerExecutor,
    n_iterations: int = 10000,
    polling_interval_seconds: float = 0.01,
) -> None:
    message = storage.read_output(Loc().exterior(), "body")
    graph = ptype_from_bytes(message, GraphData)

    output_idx = graph.output_idx()
    if output_idx is None:
        raise ValueError("Cannot resume a graph with no Output node.")

    for _ in range(n_iterations):
        walk_results = walk_node(storage, Loc(), output_idx, graph)
        if walk_results.errored != []:
            # TODO: add to base class after storage refactor
            (storage.logs_path.parent / "-" / "_error").touch()
            node_errors = "\n".join(str(x) for x in walk_results.errored)
            storage.write_node_errors(Loc(), node_errors)

            print("\n\nGraph finished with errors.\n\n")

            for error_loc in walk_results.errored:
                print(error_loc)
                print(storage.read_errors(error_loc))
                print("\n\n")

            print("--- Tierkreis graph errors above this line. ---\n\n")
            break

        start_nodes(storage, executor, walk_results.inputs_ready)
        if storage.is_node_finished(Loc()):
            break
        sleep(polling_interval_seconds)
