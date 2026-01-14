"""Implementation to access node storage data."""

from tierkreis.builder import GraphBuilder
from tierkreis.controller.data.graph import GraphData
from tierkreis.controller.data.location import Loc
from tierkreis.controller.data.types import PType, ptype_from_bytes
from tierkreis.controller.storage.filestorage import (
    ControllerFileStorage as FileStorage,
)
from tierkreis.controller.storage.in_memory import (
    ControllerInMemoryStorage as InMemoryStorage,
)
from tierkreis.controller.storage.protocol import ControllerStorage
from tierkreis.exceptions import TierkreisError

__all__ = ["FileStorage", "InMemoryStorage"]


def read_outputs(
    graph: GraphData | GraphBuilder,
    storage: ControllerStorage,
) -> dict[str, PType] | PType:
    """Read the outputs of a workflow graph.

    :param graph: The graph to read.
    :type graph: GraphData | GraphBuilder
    :param storage: The storage of the workflow run.
    :type storage: ControllerStorage
    :return: The output values. If the graph has a single output port named "value" it
        is returned directly, otherwise a dictionary mapping output port names to their
        values is returned.
    :rtype: dict[str, PType] | PType
    """
    if isinstance(graph, GraphBuilder):
        graph = graph.get_data()

    out_ports = list(graph.nodes[graph.output_idx()].inputs.keys())
    if len(out_ports) == 1 and "value" in out_ports:
        return ptype_from_bytes(storage.read_output(Loc(), "value"))
    return {k: ptype_from_bytes(storage.read_output(Loc(), k)) for k in out_ports}


def read_loop_trace(
    graph: GraphData | GraphBuilder,
    storage: ControllerStorage,
    node_name: str,
    output_name: str | None = None,
) -> list[PType | dict[str, list[PType]]]:
    """Read the trace of a named loop.

    This is useful to track intermediate values in a loop.

    :param graph: The graph to read.
    :type graph: GraphData | GraphBuilder
    :param storage: The storage of the workflow run.
    :type storage: ControllerStorage
    :param node_name: The name of the loop node.
    :type node_name: str
    :param output_name: The name of the output port to trace, defaults to None
    :type output_name: str | None, optional
    :raises TierkreisError: If the loop name is not found in debug data.
    :raises TierkreisError: If the output name is not found in loop node output.
    :return: A list of traced values. If output_name is None, each entry is a dict
        mapping output port names to their values at each iteration, otherwise a list
        of values for the specified output port is returned.
    :rtype: list[PType | dict[str, list[PType]]]
    """
    if isinstance(graph, GraphBuilder):
        graph = graph.get_data()
    loc = storage.loc_from_node_name(node_name)
    if loc is None:
        msg = f"Loop name {node_name} not found in debug data."
        raise TierkreisError(msg)
    output_names = storage.read_output_ports(loc)
    if output_name is None:
        traces = {
            name: [ptype_from_bytes(r) for r in storage.read_loop_trace(loc, name)]
            for name in output_names
            if name != "should_continue"
        }
        return [
            dict(zip(traces.keys(), vals, strict=False))
            for vals in zip(*traces.values(), strict=False)
        ]

    if output_name not in output_names:
        msg = f"Output name {output_name} not found in loop node output"
        raise TierkreisError(msg)
    results = storage.read_loop_trace(loc, output_name)
    return [ptype_from_bytes(r) for r in results]
