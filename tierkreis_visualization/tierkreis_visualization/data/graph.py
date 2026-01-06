from typing import assert_never
from fastapi import HTTPException
from tierkreis.controller.data.location import Loc
from tierkreis.controller.storage.protocol import ControllerStorage
from tierkreis_visualization.data.eval import get_eval_node
from tierkreis_visualization.data.loop import get_loop_node
from tierkreis_visualization.data.map import get_map_node
from tierkreis_visualization.routers.models import PyGraph


def parse_node_location(node_location_str: str) -> Loc:
    return Loc(node_location_str)


def get_errored_nodes(storage: ControllerStorage) -> list[Loc]:
    errored_nodes = storage.read_errors(Loc("-"))
    return [parse_node_location(node) for node in errored_nodes.split("\n")]


def get_node_data(storage: ControllerStorage, loc: Loc) -> PyGraph:
    errored_nodes = get_errored_nodes(storage)

    try:
        node = storage.read_node_def(loc)
    except FileNotFoundError:
        raise HTTPException(404, detail="Node definition not found.")

    match node.type:
        case "eval":
            data = get_eval_node(storage, loc, errored_nodes)
            return PyGraph(nodes=data.nodes, edges=data.edges)

        case "loop":
            data = get_loop_node(storage, loc, errored_nodes)
            return PyGraph(nodes=data.nodes, edges=data.edges)

        case "map":
            data = get_map_node(storage, loc, node, errored_nodes)
            return PyGraph(nodes=data.nodes, edges=data.edges)

        case "function" | "const" | "ifelse" | "eifelse" | "input" | "output":
            raise HTTPException(
                400, detail="Only eval, loop and map nodes return a graph."
            )

        case _:
            assert_never(node)
