from tierkreis.builder import GraphBuilder
from tierkreis.controller.data.graph import GraphData
from tierkreis.controller.data.location import Loc
from tierkreis.controller.data.types import PType, ptype_from_bytes
from tierkreis.controller.storage.protocol import ControllerStorage
from tierkreis.controller.storage.filestorage import (
    ControllerFileStorage as FileStorage,
)
from tierkreis.controller.storage.in_memory import (
    ControllerInMemoryStorage as InMemoryStorage,
)

__all__ = ["FileStorage", "InMemoryStorage"]


def read_outputs(
    g: GraphData | GraphBuilder, storage: ControllerStorage
) -> dict[str, PType] | PType:
    if isinstance(g, GraphBuilder):
        g = g.get_data()

    out_ports = list(g.nodes[g.output_idx()].inputs.keys())
    if len(out_ports) == 1 and "value" in out_ports:
        return ptype_from_bytes(storage.read_output(Loc(), "value"))
    return {k: ptype_from_bytes(storage.read_output(Loc(), k)) for k in out_ports}
