from tierkreis.builder import GraphBuilder
from tierkreis.controller.data.location import Loc
from tierkreis.controller.data.types import PType, ptype_from_bytes
from tierkreis.controller.storage.protocol import ControllerStorage
from tierkreis.controller.storage.filestorage import (
    ControllerFileStorage as FileStorage,
)
from tierkreis.controller.storage.in_memory import (
    ControllerInMemoryStorage as InMemoryStorage,
)
from tierkreis.exceptions import TierkreisError
from tierkreis_core import GraphData

__all__ = ["FileStorage", "InMemoryStorage"]


def read_outputs(
    g: GraphData | GraphBuilder, storage: ControllerStorage
) -> dict[str, PType] | PType:
    if isinstance(g, GraphBuilder):
        g = g.get_data()

    output_idx = g.output_idx()
    if output_idx is None:
        raise ValueError("Cannot read outputs of a graph with no Output node.")

    out_ports = list(g.get_nodedef(output_idx).inputs.keys())
    if len(out_ports) == 1 and "value" in out_ports:
        return ptype_from_bytes(storage.read_output(Loc(), "value"))
    return {k: ptype_from_bytes(storage.read_output(Loc(), k)) for k in out_ports}


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
        raise TierkreisError(f"Loop name {node_name} not found in debug data.")
    output_names = storage.read_output_ports(loc)
    if output_name is None:
        traces = {
            name: [ptype_from_bytes(r) for r in storage.read_loop_trace(loc, name)]
            for name in output_names
            if name != "should_continue"
        }
        return [dict(zip(traces.keys(), vals)) for vals in zip(*traces.values())]

    if output_name not in output_names:
        raise TierkreisError(f"Output name {output_name} not found in loop node output")
    results = storage.read_loop_trace(loc, output_name)
    return [ptype_from_bytes(r) for r in results]
