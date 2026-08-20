from typing import assert_never

from tierkreis.controller.data.graph import (
    GraphData,
    graph_node_from_loc,
    in_edges,
)
from tierkreis.controller.data.location import Loc
from tierkreis.controller.data.types import ptype_from_bytes
from tierkreis.controller.storage.exceptions import EntryNotFoundError
from tierkreis.controller.storage.protocol import ControllerStorage
from tierkreis.exceptions import TierkreisError

from tierkreis_visualization.data.models import NodeInputs, NodeStatus, PyEdge, PyNode
from tierkreis_visualization.data.outputs import outputs_from_loc, task_inputs
from tierkreis_visualization.routers.models import PyGraph


def node_status(
    storage: ControllerStorage,
    node_location: Loc,
    errored_nodes: list[Loc],
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


def get_eval_node(
    storage: ControllerStorage,
    node_location: Loc,
    errored_nodes: list[Loc],
) -> PyGraph:
    try:
        thunk = storage.read_output(node_location.N(-1), "body")
        graph = ptype_from_bytes(thunk, GraphData)
    except (EntryNotFoundError, TierkreisError):
        thunk = storage.read_output(Loc("-.N-1"), "body")
        graph = ptype_from_bytes(thunk, GraphData)
        graph = graph_node_from_loc(node_location, graph)[1]

    pynodes: list[PyNode] = []
    py_edges: list[PyEdge] = []
    hidden_nodes: set[Loc] = set()
    for i, node in enumerate(graph.nodes):
        new_location = node_location.N(i)

        status = node_status(storage, new_location, errored_nodes)
        started_time = storage.read_started_time(new_location) or ""
        finished_time = storage.read_finished_time(new_location) or ""
        if i in graph.node_metadata:
            is_hidden = graph.node_metadata[i].is_hidden
        else:
            is_hidden = False

        value: str | None = None
        inputs: list[NodeInputs] = []
        match node.type:
            case "function":
                name = node.function_name
                inputs = task_inputs(storage, new_location)
            case "map" | "eval" | "loop" | "ifelse" | "eifelse":
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
            inputs=inputs,
        )
        if not is_hidden:
            pynodes.append(pynode)
        else:
            hidden_nodes.add(new_location)

        for p0, (idx, p1) in in_edges(node).items():
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

    # Rewire edges through hidden nodes
    # Works based on the assumption that hidden nodes have single input and output
    # and that no two hidden nodes are directly connected.
    # Currently only used for fold/unfold nodes, which follow this pattern.
    # In this case, at most one of the edges holds a value, so we can just take it for the new edge.
    rewired_edges: list[PyEdge] = []
    for edge in py_edges:
        if edge.to_node in hidden_nodes:
            next_edge = next(filter(lambda e: e.from_node == edge.to_node, py_edges))
            rewired_edges.append(
                PyEdge(
                    from_node=edge.from_node,
                    from_port=edge.from_port,
                    to_node=next_edge.to_node,
                    to_port=next_edge.to_port,
                    value=edge.value if edge.value is not None else next_edge.value,
                )
            )
        elif edge.from_node not in hidden_nodes:
            rewired_edges.append(edge)

    return PyGraph(nodes=pynodes, edges=rewired_edges)
