from pydantic import BaseModel
from tierkreis.controller.storage.adjacency import outputs_iter
from tierkreis.controller.storage.protocol import ControllerStorage
from tierkreis.exceptions import TierkreisError
from tierkreis_visualization.data.eval import check_error
from tierkreis_visualization.data.models import PyEdge, PyNode
from tierkreis_core import Loc, NodeDef


class MapNodeData(BaseModel):
    nodes: list[PyNode]
    edges: list[PyEdge]


def get_map_node(
    storage: ControllerStorage, loc: Loc, map: NodeDef.Map, errored_nodes: list[Loc]
) -> MapNodeData:
    parent = loc.parent()
    if parent is None:
        raise TierkreisError("MAP node must have parent.")

    first_ref = next(x for x in map.inputs.values() if x.port_id == "*")
    map_eles = outputs_iter(storage, parent.extend_from_ref(first_ref))
    description = storage.read_node_description(loc.M(0))
    nodes: list[PyNode] = []
    for i, ele in map_eles:
        node = PyNode(
            id=loc.M(i),
            status="Started",
            function_name=ele,
            node_location=loc.M(i),
            node_type="eval",
            started_time=storage.read_started_time(loc.M(i)) or "",
            finished_time=storage.read_finished_time(loc.M(i)) or "",
            outputs=list(description.outputs),
        )
        if check_error(loc.M(i), errored_nodes):
            node.status = "Error"
        elif storage.is_node_finished(loc.M(i)):
            node.status = "Finished"
        nodes.append(node)

    return MapNodeData(nodes=nodes, edges=[])
