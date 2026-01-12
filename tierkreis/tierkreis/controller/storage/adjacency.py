import logging

from tierkreis.controller.data.location import Loc
from tierkreis.controller.storage.protocol import ControllerStorage
from tierkreis_core import ValueRef, NodeDef, PortID

logger = logging.getLogger(__name__)


def unfinished_inputs(
    storage: ControllerStorage, loc: Loc, node: NodeDef
) -> list[ValueRef]:
    """Retrieve the ValueRefs for each edge of the graph that isn't finished.

    As ExteriorRefs always refer to edges that are finished they can be safely ingored.
    """
    ins = node.in_edges.values()
    ins = [
        x for x in ins if isinstance(x, ValueRef)
    ]  # ExteriorRefs are always finished.
    return [x for x in ins if not storage.is_node_finished(loc.N(x.node_index))]


def outputs_iter(storage: ControllerStorage, loc: Loc) -> list[tuple[int, PortID]]:
    """Retrieve the indexes for a virtual map node by parsing output port names."""
    eles = storage.read_output_ports(loc)
    return [(int(x.split("-")[-1]), x) for x in eles]
