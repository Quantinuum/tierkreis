"""Graph information based on adjacency."""

import logging

from tierkreis.controller.data.core import PortID, ValueRef
from tierkreis.controller.data.graph import NodeDef, in_edges
from tierkreis.controller.data.location import Loc
from tierkreis.controller.storage.protocol import ControllerStorage

logger = logging.getLogger(__name__)


def unfinished_inputs(
    storage: ControllerStorage,
    loc: Loc,
    node: NodeDef,
) -> list[ValueRef]:
    """Find the unfinished inputs of a node.

    :param storage: The storage to write from.
    :type storage: ControllerStorage
    :param loc: The node location to check for.
    :type loc: Loc
    :param node: The node definition containing the output names.
    :type node: NodeDef
    :return: A list of references to node inputs.
    :rtype: list[ValueRef]
    """
    # ifelse is lazy: only wait for pred before starting
    ins = [node.pred] if node.type == "ifelse" else in_edges(node).values()
    ins = [x for x in ins if x[0] >= 0]  # inputs at -1 already finished they're linked
    return [x for x in ins if not storage.is_node_finished(loc.N(x[0]))]


def outputs_iter(storage: ControllerStorage, loc: Loc) -> list[tuple[int, PortID]]:
    """Find all the outputs of a node and provide them with their index as map elements.

    This is only used in map nodes to go from the * port to the values
    of actual map elements.
    This can be from an unfold where we get (index, index)
    or map (index, "eval_output_name-index")

    :param storage: The storage to read from.
    :type storage: ControllerStorage
    :param loc: The location to get the outputs from.
    :type loc: Loc
    :return: A tuple of (index, portname) of
    :rtype: list[tuple[int, PortID]]
    """
    eles = storage.read_output_ports(loc)
    return [(int(x.split("-")[-1]), x) for x in eles]
