import json
from typing import Optional

from tierkreis.controller.data.types import ptype_from_bytes
from tierkreis.controller.storage.protocol import ControllerStorage
from tierkreis_core import GraphData, Loc, NodeDef, NodeDescription

from tierkreis.exceptions import TierkreisError
from tierkreis_visualization.data.models import PyNode, NodeStatus, PyEdge
from tierkreis_visualization.data.outputs import outputs_from_loc
from tierkreis_visualization.routers.models import PyGraph


def node_status(
    is_finished: bool, definition: Optional[NodeDescription], has_error: bool = False
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
    i: int,
    node: NodeDef.IfElse | NodeDef.EagerIfElse,
    py_edges: list[PyEdge],
):
    try:
        pred = json.loads(
            storage.read_output(loc.N(node.pred.node_index), node.pred.port_id)
        )
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
    thunk = storage.read_output(node_location.exterior(), "body")
    graph = ptype_from_bytes(thunk, GraphData)

    pynodes: list[PyNode] = []
    py_edges: list[PyEdge] = []

    for i, node in enumerate(graph.nodes):
        new_location = node_location.N(i)
        is_finished = storage.is_node_finished(new_location)
        has_error = check_error(new_location, errored_nodes)
        try:
            definition = storage.read_node_description(new_location)
        except (FileNotFoundError, TierkreisError):
            definition = None

        status = node_status(is_finished, definition, has_error)
        started_time = storage.read_started_time(new_location) or ""
        finished_time = storage.read_finished_time(new_location) or ""
        value: str | None = None
        node_type: str
        match node:
            case NodeDef.Func():
                name = node.name
                node_type = "function"
            case NodeDef.IfElse():
                name = "ifelse"
                node_type = "ifelse"
                add_conditional_edges(storage, node_location, i, node, py_edges)
            case NodeDef.Map():
                name = "map"
                node_type = "map"
            case NodeDef.Eval():
                name = "eval"
                node_type = "eval"
            case NodeDef.Loop():
                name = "loop"
                node_type = "loop"
            case NodeDef.EagerIfElse():
                name = "eifelse"
                node_type = "eifelse"
            case NodeDef.Const():
                name = "const"
                node_type = "const"
                value = outputs_from_loc(storage, node_location.N(i), "value")
            case NodeDef.Output():
                name = "output"
                node_type = "output"
                in_edges = node.in_edges
                if len(in_edges) == 1:
                    ref = next(iter(in_edges.values()))
                    try:
                        value = outputs_from_loc(
                            storage, node_location.extend_from_ref(ref), ref.port_id
                        )
                    except (FileNotFoundError, TierkreisError):
                        value = None
            case NodeDef.Input():
                name = "input"
                node_type = "input"
                value = node.name
            case _:
                raise ValueError(f"Unhandled NodeDef of type: {type(node)}")

        pynode = PyNode(
            id=new_location,
            status=status,
            function_name=name,
            node_location=new_location,
            node_type=node_type,
            value=value,
            started_time=started_time,
            finished_time=finished_time,
            outputs=list(definition.outputs) if definition is not None else [],
        )
        pynodes.append(pynode)

        for p0, (idx, p1) in node.in_edges.items():
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
