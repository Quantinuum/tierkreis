"""Implementation to access node storage data."""

from tierkreis.controller.data.graph import GraphData
from tierkreis.controller.data.location import Loc
from tierkreis.controller.data.models import TModel
from tierkreis.controller.data.types import PType, is_optional, ptype_from_bytes
from tierkreis.controller.storage.exceptions import EntryNotFoundError
from tierkreis.controller.storage.filestorage import (
    ControllerFileStorage as FileStorage,
)
from tierkreis.controller.storage.in_memory import (
    ControllerInMemoryStorage as InMemoryStorage,
)
from tierkreis.controller.storage.protocol import ControllerStorage
from tierkreis.exceptions import TierkreisError
from tierkreis.models import Workflow

__all__ = ["FileStorage", "InMemoryStorage"]


def _read_output(
    storage: ControllerStorage,
    port_name: str,
    annotation: type | None,
) -> PType:
    """Try to get the output `port_name` from the root graph.

    If `annotation` indicates that the value is optional
    then do not raise on EntryNotFound.
    """
    try:
        return ptype_from_bytes(storage.read_output(Loc(), port_name))
    except EntryNotFoundError as exc:
        if annotation and is_optional(annotation):
            return None
        msg = f"Output {port_name} not found."
        raise TierkreisError(msg) from exc


def read_outputs[A: TModel, B: TModel](
    graph: GraphData | Workflow[A, B],
    storage: ControllerStorage,
) -> dict[str, PType] | PType:
    """Read the outputs of a workflow graph.

    The bytes are parsed into Python types if possible.

    :param graph: The graph to read.
    :type graph: GraphData | Workflow
    :param storage: The storage of the workflow run.
    :type storage: ControllerStorage
    :return: The output values. If the graph has a single output port named "value" it
        is returned directly, otherwise a dictionary mapping output port names to their
        values is returned.
    :rtype: dict[str, PType] | PType
    """
    output_annotation = None
    if isinstance(graph, Workflow):
        output_annotation = graph.outputs_type
        graph = graph.data

    out_ports = list(graph.nodes[graph.output_idx()].inputs.keys())

    if len(out_ports) == 1 and "value" in out_ports:
        return _read_output(storage, "value", output_annotation)

    outputs: dict[str, PType] = {}
    for port_name in out_ports:
        port_annotation = (
            output_annotation.__annotations__.get(port_name)
            if output_annotation
            else None
        )
        outputs[port_name] = _read_output(storage, port_name, port_annotation)

    return outputs


def read_loop_trace(
    graph: GraphData | Workflow,
    storage: ControllerStorage,
    node_name: str,
    output_name: str | None = None,
) -> list[PType | dict[str, list[PType]]]:
    """Read the trace of a named loop.

    This is useful to track intermediate values in a loop.

    :param graph: The graph to read.
    :type graph: GraphData | Workflow
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
    if isinstance(graph, Workflow):
        graph = graph.data
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
