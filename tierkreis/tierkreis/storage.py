from tierkreis.builder import GraphBuilder
from tierkreis.controller.data.graph import GraphData
from tierkreis.controller.data.location import Loc
from tierkreis.controller.data.models import TModel
from tierkreis.controller.data.types import PType, is_optional, ptype_from_bytes
from tierkreis.controller.storage.exceptions import EntryNotFound
from tierkreis.controller.storage.filestorage import (
    ControllerFileStorage as FileStorage,
)
from tierkreis.controller.storage.in_memory import (
    ControllerInMemoryStorage as InMemoryStorage,
)
from tierkreis.controller.storage.protocol import ControllerStorage
from tierkreis.exceptions import TierkreisError

__all__ = ["FileStorage", "InMemoryStorage"]


def _read_output(
    storage: ControllerStorage,
    port_name: str,
    annotation: type | None,
) -> PType:
    """Tries to get the output `port_name` from the root graph.
    If `annotation` indicates that the value is optional then do not raise on EntryNotFound.
    """
    try:
        return ptype_from_bytes(storage.read_output(Loc(), port_name))
    except EntryNotFound as exc:
        if annotation and is_optional(annotation):
            return None
        msg = f"Output {port_name} not found."
        raise TierkreisError(msg) from exc


def read_outputs[A: TModel, B: TModel](
    g: GraphData | GraphBuilder[A, B],
    storage: ControllerStorage,
) -> dict[str, PType] | PType:
    """Read the outputs from the `storage`.

    The bytes are parsed into Python types if possible.
    """
    output_annotation = None
    if isinstance(g, GraphBuilder):
        output_annotation = g.outputs_type
        g = g.get_data()

    out_ports = list(g.nodes[g.output_idx()].inputs.keys())

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
    g: GraphData | GraphBuilder,
    storage: ControllerStorage,
    node_name: str,
    output_name: str | None = None,
) -> list[PType | dict[str, list[PType]]]:
    """Reads the trace of a loop from storage."""
    if isinstance(g, GraphBuilder):
        g = g.get_data()
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
        return [dict(zip(traces.keys(), vals, strict=False)) for vals in zip(*traces.values(), strict=False)]

    if output_name not in output_names:
        msg = f"Output name {output_name} not found in loop node output"
        raise TierkreisError(msg)
    results = storage.read_loop_trace(loc, output_name)
    return [ptype_from_bytes(r) for r in results]
