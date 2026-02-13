import json
from typing import assert_never

from tierkreis.controller.data.core import NodeIndex
from tierkreis.controller.data.location import Loc
from tierkreis.controller.data.graph import GraphData, IfElse
from tierkreis.controller.data.types import ptype_from_bytes
from tierkreis.controller.storage.adjacency import in_edges
from tierkreis.controller.storage.protocol import ControllerStorage

from tierkreis.exceptions import TierkreisError
from tierkreis_visualization.data.models import PyNode, NodeStatus, PyEdge
from tierkreis_visualization.data.outputs import outputs_from_loc
from tierkreis_visualization.routers.models import PyGraph


def node_status(
    storage: ControllerStorage, node_location: Loc, errored_nodes: list[Loc]
) -> NodeStatus:
    if storage.is_node_finished(node_location):
        return "Finished"

    if storage.is_node_started(node_location):
        if check_error(node_location, errored_nodes):
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
            value = outputs_from_loc(storage, loc.N(idx), p)
        except FileNotFoundError:
            value = None
        edge = PyEdge(
            from_node=loc.N(idx),
            from_port=p,
            to_node=loc.N(i),
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

        status = node_status(storage, new_location, errored_nodes)
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
                value = outputs_from_loc(storage, node_location.N(i), "value")
            case "output":
                name = node.type
                if len(node.inputs) == 1:
                    (idx, p) = next(iter(node.inputs.values()))
                    try:
                        value = outputs_from_loc(storage, node_location.N(idx), p)
                    except (FileNotFoundError, TierkreisError):
                        value = None
            case "input":
                name = node.type
                value = node.name
            case _:
                assert_never(node)

        pynode = PyNode(
            id=new_location,
            status=status,
            function_name=name,
            node_location=new_location,
            node_type=node.type,
            value=value,
            started_time=started_time,
            finished_time=finished_time,
            outputs=list(node.outputs),
        )
        pynodes.append(pynode)

        for p0, (idx, p1) in in_edges(node).items():
            value: str | None = None

            try:
                value = outputs_from_loc(storage, node_location.N(idx), p1)
            except (FileNotFoundError, TierkreisError, UnicodeDecodeError):
                value = None

            py_edge = PyEdge(
                from_node=node_location.N(idx),
                from_port=p1,
                to_node=node_location.N(i),
                to_port=p0,
                value=value,
            )
            py_edges.append(py_edge)

    return PyGraph(nodes=pynodes, edges=py_edges)
