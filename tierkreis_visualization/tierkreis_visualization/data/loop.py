import json
from pydantic import BaseModel
from tierkreis.controller.data.location import Loc
from tierkreis.controller.storage.protocol import ControllerStorage


from tierkreis_visualization.data.eval import check_error
from tierkreis_visualization.data.models import PyNode, PyEdge


class LoopNodeData(BaseModel):
    nodes: list[PyNode]
    edges: list[PyEdge]


def get_loop_node(
    storage: ControllerStorage, node_location: Loc, errored_nodes: list[Loc]
) -> LoopNodeData:
    i = 0
    while storage.is_node_started(node_location.L(i + 1)):
        i += 1
    new_location = node_location.L(i)

    nodes = [
        PyNode(
            id=node_location.N(n),
            status="Finished",
            function_name=f"L{n}",
            node_location=node_location.L(n),
            started_time=storage.read_started_time(node_location.L(n)) or "",
            finished_time=storage.read_finished_time(node_location.L(n)) or "",
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
            id=node_location.N(i),
            status=last_status,
            function_name=f"L{i}",
            node_location=new_location,
            started_time=storage.read_started_time(new_location) or "",
            finished_time=storage.read_finished_time(new_location) or "",
        )
    )
    edges = []
    for port_name in storage.read_node_def(node_location.L(0)).outputs:
        edges.extend(
            [
                PyEdge(
                    from_node=node_location.N(n),
                    from_port=port_name,
                    to_node=node_location.N(n + 1),
                    to_port=port_name,
                    value=json.loads(
                        storage.read_output(node_location.L(n), port_name)
                    ),
                )
                for n in range(i)
            ]
        )
    return LoopNodeData(nodes=nodes, edges=edges)
