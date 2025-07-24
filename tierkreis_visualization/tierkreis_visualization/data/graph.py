from fastapi import HTTPException
from tierkreis.controller.data.location import Loc
from tierkreis.controller.storage.protocol import ControllerStorage
from tierkreis_core import NodeDef

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
        description = storage.read_node_description(loc)
    except FileNotFoundError:
        raise HTTPException(404, detail="Node definition not found.")

    match description.definition:
        case NodeDef.Eval():
            data = get_eval_node(storage, loc, errored_nodes)
            return PyGraph(nodes=data.nodes, edges=data.edges)

        case NodeDef.Loop():
            data = get_loop_node(storage, loc, errored_nodes)
            return PyGraph(nodes=data.nodes, edges=data.edges)

        case NodeDef.Map():
            data = get_map_node(storage, loc, description.definition, errored_nodes)
            return PyGraph(nodes=data.nodes, edges=data.edges)

        case (
            NodeDef.Func
            | NodeDef.Const
            | NodeDef.IfElse
            | NodeDef.EagerIfElse
            | NodeDef.Input
            | NodeDef.Output
        ):
            raise HTTPException(
                400, detail="Only eval, loop and map nodes return a graph."
            )

        case _:
            raise ValueError(
                f"Unhandled NodeDef of type: {type(description.definition)}"
            )
