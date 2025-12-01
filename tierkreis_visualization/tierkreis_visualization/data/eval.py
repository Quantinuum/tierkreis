import json
from typing import Optional, assert_never

from pydantic import BaseModel
from tierkreis.controller.data.core import NodeIndex
from tierkreis.controller.data.location import Loc
from tierkreis.controller.data.graph import GraphData, IfElse, NodeDef
from tierkreis.controller.data.types import ptype_from_bytes
from tierkreis.controller.storage.adjacency import in_edges
from tierkreis.controller.storage.protocol import ControllerStorage

from tierkreis.exceptions import TierkreisError
from tierkreis_visualization.data.models import PyNode, NodeStatus, PyEdge
from tierkreis_visualization.routers.models import PyGraph


def node_status(
    is_finished: bool, definition: Optional[NodeDef], has_error: bool = False
) -> NodeStatus:
    if is_finished:
        return "Finished"

    if definition is not None:
        if has_error:
            return "Error"
        return "Started"

    return "Not started"


def check_error(node_location: Loc, errored_nodes: list[Loc]) -> bool:
    return any(node.startswith(node_location) for node in errored_nodes)


def add_conditional_edges(
    storage: ControllerStorage,
    loc: Loc,
    i: NodeIndex,
    node: IfElse,
    py_edges: list[PyEdge],
):
    try:
        pred = json.loads(storage.read_output(loc.N(node.pred[0]), node.pred[1]))
    except (FileNotFoundError, TierkreisError):
        pred = None

    refs = {True: node.if_true, False: node.if_false}

    for branch, (idx, p) in refs.items():
        try:
            value = json.loads(storage.read_output(loc.N(idx), p))
        except FileNotFoundError:
            value = None
        edge = PyEdge(
            from_node=idx,
            from_port=p,
            to_node=i,
            to_port=f"If{branch}",
            conditional=pred is None or pred != branch,
            value=value,
        )
        py_edges.append(edge)


def get_eval_node(
    storage: ControllerStorage, node_location: Loc, errored_nodes: list[Loc]
) -> PyGraph:
    thunk = storage.read_output(node_location.N(-1), "body")
    graph = ptype_from_bytes(thunk, GraphData)

    pynodes: list[PyNode] = []
    py_edges: list[PyEdge] = []

    for i, node in enumerate(graph.nodes):
        new_location = node_location.N(i)
        is_finished = storage.is_node_finished(new_location)
        has_error = check_error(new_location, errored_nodes)
        try:
            definition = storage.read_node_def(new_location)
        except (FileNotFoundError, TierkreisError):
            definition = None

        status = node_status(is_finished, definition, has_error)
        started_time = storage.read_started_time(new_location) or ""
        finished_time = storage.read_finished_time(new_location) or ""
        value: str | None = None
        match node.type:
            case "function":
                name = node.function_name
            case "ifelse":
                name = node.type
                add_conditional_edges(storage, node_location, i, node, py_edges)
            case "map" | "eval" | "loop" | "eifelse":
                name = node.type
            case "const":
                name = node.type
                value = node.value
            case "output":
                name = node.type
                if len(node.inputs) == 1:
                    (idx, p) = next(iter(node.inputs.values()))
                    try:
                        value = json.loads(storage.read_output(node_location.N(idx), p))
                    except (FileNotFoundError, TierkreisError):
                        value = None
            case "input":
                name = node.type
                value = node.name
            case _:
                assert_never(node)

        pynode = PyNode(
            id=i,
            status=status,
            function_name=name,
            node_location=new_location,
            value=value,
            started_time=started_time,
            finished_time=finished_time,
        )
        pynodes.append(pynode)

        for p0, (idx, p1) in in_edges(node).items():
            value = None

            try:
                value = storage.read_output(node_location.N(idx), p1).decode()
            except (FileNotFoundError, TierkreisError, UnicodeDecodeError):
                value = None

            py_edge = PyEdge(
                from_node=idx, from_port=p1, to_node=i, to_port=p0, value=value
            )
            py_edges.append(py_edge)

    return PyGraph(nodes=pynodes, edges=py_edges)
