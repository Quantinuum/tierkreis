from pydantic import BaseModel
from tierkreis.controller.data.graph import GraphData, Loop
from tierkreis.controller.data.location import Loc
from tierkreis.controller.data.types import ptype_from_bytes
from tierkreis.controller.storage.protocol import ControllerStorage
from tierkreis.exceptions import TierkreisError

from tierkreis_visualization.data.eval import check_error
from tierkreis_visualization.data.models import PyNode, PyEdge
from tierkreis_visualization.data.outputs import outputs_from_loc


class LoopNodeData(BaseModel):
    nodes: list[PyNode]
    edges: list[PyEdge]


def get_loop_node(
    storage: ControllerStorage, node_location: Loc, node: Loop, errored_nodes: list[Loc]
) -> LoopNodeData:
    parent = node_location.parent()
    if parent is None:
        raise TierkreisError("LOOP node must have parent.")

    i = 0
    while storage.is_node_started(node_location.L(i + 1)):
        i += 1
    new_location = node_location.L(i)

    # Get the outputs from the graph fed into the Loop's body input.
    graph_input = ptype_from_bytes(
        storage.read_output(parent.N(node.body[0]), node.body[1]), GraphData
    )
    outputs = graph_input.output_ports

    nodes = [
        PyNode(
            id=node_location.L(n),
            status="Finished",
            function_name=f"L{n}",
            node_location=node_location.L(n),
            node_type="eval",
            started_time=storage.read_started_time(node_location.L(n)) or "",
            finished_time=storage.read_finished_time(node_location.L(n)) or "",
            outputs=outputs,
        )
        for n in range(i)
    ]

    if check_error(node_location, errored_nodes):
        last_status = "Error"
    elif storage.is_node_finished(new_location):
        last_status = "Finished"
    else:
        last_status = "Started"
    nodes.append(
        PyNode(
            id=new_location,
            status=last_status,
            function_name=f"L{i}",
            node_location=new_location,
            node_type="eval",
            started_time=storage.read_started_time(new_location) or "",
            finished_time=storage.read_finished_time(new_location) or "",
            outputs=list(outputs),
        )
    )
    edges = []
    for port_name in outputs:
        edges.extend(
            [
                PyEdge(
                    from_node=node_location.L(n),
                    from_port=port_name,
                    to_node=node_location.L(n + 1),
                    to_port=port_name,
                    value=outputs_from_loc(storage, node_location.L(n), port_name),
                )
                for n in range(i)
            ]
        )
    return LoopNodeData(nodes=nodes, edges=edges)
