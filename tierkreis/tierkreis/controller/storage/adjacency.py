import logging
from typing import assert_never

from tierkreis.controller.data.core import PortID, ValueRef
from tierkreis.controller.data.graph import (
    NodeDef,
)
from tierkreis.controller.data.location import Loc
from tierkreis.controller.storage.protocol import ControllerStorage

logger = logging.getLogger(__name__)


def in_edges(node: NodeDef) -> dict[PortID, ValueRef]:
    parents = {k: v for k, v in node.inputs.items()}

    match node.type:
        case "eval":
            parents["body"] = node.graph
        case "loop":
            parents["body"] = node.body
        case "map":
            parents["body"] = node.body
        case "ifelse":
            parents["pred"] = node.pred
        case "eifelse":
            parents["pred"] = node.pred
            parents["body_true"] = node.if_true
            parents["body_false"] = node.if_false
        case "const" | "function" | "input" | "output":
            pass
        case _:
            assert_never(node)

    return parents


def unfinished_inputs(
    storage: ControllerStorage, loc: Loc, node: NodeDef
) -> list[ValueRef]:
    ins = in_edges(node).values()
    ins = [x for x in ins if x[0] >= 0]  # inputs at -1 already finished
    return [x for x in ins if not storage.is_node_finished(loc.N(x[0]))]


def outputs_iter(storage: ControllerStorage, loc: Loc) -> list[tuple[int, PortID]]:
    eles = storage.read_output_ports(loc)
    return [(int(x.split("-")[-1]), x) for x in eles]
