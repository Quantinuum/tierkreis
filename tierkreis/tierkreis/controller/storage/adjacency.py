import logging

from tierkreis.controller.data.core import PortID, ValueRef
from tierkreis.controller.data.graph import NodeDef, in_edges
from tierkreis.controller.data.location import Loc
from tierkreis.controller.storage.protocol import ControllerStorage

logger = logging.getLogger(__name__)


def unfinished_inputs(
    storage: ControllerStorage, loc: Loc, node: NodeDef
) -> list[ValueRef]:
    raw_ins = in_edges(node)
    if node.type == "ifelse":  # ifelse is lazy: only wait for pred before starting
        _ = raw_ins.pop("body_true", None)
        _ = raw_ins.pop("body_false", None)
    ins = [x for x in raw_ins.values() if x[0] >= 0]  # inputs at -1 already finished
    return [x for x in ins if not storage.is_node_finished(loc.N(x[0]))]


def outputs_iter(storage: ControllerStorage, loc: Loc) -> list[tuple[int, PortID]]:
    eles = storage.read_output_ports(loc)
    return [(int(x.split("-")[-1]), x) for x in eles]
