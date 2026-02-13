from typing import assert_never
from fastapi import HTTPException
from tierkreis.controller.data.graph import Eval, GraphData
from tierkreis.controller.data.location import Loc
from tierkreis.controller.data.types import ptype_from_bytes
from tierkreis.controller.storage.protocol import ControllerStorage
from tierkreis.exceptions import TierkreisError
from tierkreis_visualization.data.eval import render_graph
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

    match loc.pop_last()[0]:
        case ("M", _) | ("L", _) | "-":
            graph = storage.read_graph_def(loc)
            return render_graph(storage, loc, graph, errored_nodes)
        case ("N", _):
            pass  # Fall through below
        case x:
            assert_never(x)
    try:
        node = storage.read_node_def(loc)
    except FileNotFoundError:
        raise HTTPException(404, detail="Node definition not found.")

    match node.type:
        case "eval":
            data = get_eval_node(storage, loc, node, errored_nodes)
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


def get_eval_node(
    storage: ControllerStorage, node_location: Loc, node: Eval, errored_nodes: list[Loc]
) -> PyGraph:
    parent = node_location.parent()
    if parent is None:
        raise TierkreisError("Eval node must have parent.")

    thunk = storage.read_output(parent.N(node.graph[0]), node.graph[1])
    graph = ptype_from_bytes(thunk, GraphData)
    return render_graph(storage, node_location, graph, errored_nodes)
