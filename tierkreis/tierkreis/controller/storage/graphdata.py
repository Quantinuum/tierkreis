from pathlib import Path
from uuid import UUID
from typing import Any


from pydantic import BaseModel, Field
from tierkreis.controller.data.core import PortID
from tierkreis.controller.data.graph import (
    Eval,
    GraphData,
    NodeDef,
    graph_node_from_loc,
)
from tierkreis.controller.data.location import (
    Loc,
    OutputLoc,
    WorkerCallArgs,
    get_last_index,
)
from tierkreis.controller.storage.protocol import (
    StorageEntryMetadata,
    ControllerStorage,
)
from tierkreis.exceptions import TierkreisError


class NodeData(BaseModel):
    """Internal storage class to store all necessary node information."""

    definition: NodeDef | None = None
    call_args: WorkerCallArgs | None = None
    is_done: bool = False
    has_error: bool = False
    metadata: dict[str, Any] = Field(default_factory=dict)
    error_logs: str = ""
    outputs: dict[PortID, bytes | None] = Field(default_factory=dict)
    started: str | None = None
    finished: str | None = None


class GraphDataStorage(ControllerStorage):
    def __init__(
        self,
        workflow_id: UUID,
        graph: GraphData,
        name: str | None = None,
    ) -> None:
        self.workflow_id = workflow_id
        self.name = name
        self.nodes: dict[Loc, NodeData] = {}
        self.graph = graph
        self.tkr_dir = Path.home() / ".tierkreis"

    def delete(self, path: Path) -> None:
        raise NotImplementedError("GraphDataStorage is read only storage.")

    def exists(self, path: Path) -> bool:
        raise NotImplementedError("GraphDataStorage is only for graph construction.")

    def list_subpaths(self, path: Path) -> list[Path]:
        raise NotImplementedError("GraphDataStorage uses GraphData not paths.")

    def link(self, src: Path, dst: Path) -> None:
        raise NotImplementedError("GraphDataStorage is read only storage.")

    def mkdir(self, path: Path) -> None:
        raise NotImplementedError("GraphDataStorage is read only storage.")

    def read(self, path: Path) -> bytes:
        raise NotImplementedError("GraphDataStorage uses GraphData not paths.")

    def stat(self, path: Path) -> StorageEntryMetadata:
        raise NotImplementedError("GraphDataStorage is only for graph construction.")

    def touch(self, path: Path, is_dir: bool = False) -> None:
        raise NotImplementedError("GraphDataStorage is read only storage.")

    def write(self, path: Path, value: bytes) -> None:
        raise NotImplementedError("GraphDataStorage is read only storage.")

    def write_node_def(self, node_location: Loc, node: NodeDef) -> None:
        raise NotImplementedError("GraphDataStorage is read only storage.")

    def read_node_def(self, node_location: Loc) -> NodeDef:
        try:
            if node_location.pop_last()[0][0] in ["M", "L"]:
                return Eval((-1, "body"), {})
        except (TierkreisError, TypeError):
            return Eval((-1, "body"), {})
        node, _ = graph_node_from_loc(node_location, self.graph)
        return node

    def write_worker_call_args(
        self,
        node_location: Loc,
        function_name: str,
        inputs: dict[PortID, OutputLoc],
        output_list: list[PortID],
    ) -> Path:
        raise NotImplementedError("GraphDataStorage is read only storage.")

    def read_worker_call_args(self, node_location: Loc) -> WorkerCallArgs:
        raise TierkreisError(
            f"Node location {node_location} doesn't have a associate call args."
        )

    def read_errors(self, node_location: Loc) -> str:
        return ""

    def node_has_error(self, node_location: Loc) -> bool:
        return False

    def write_node_errors(self, node_location: Loc, error_logs: str) -> None:
        raise NotImplementedError("GraphDataStorage is read only storage.")

    def mark_node_finished(self, node_location: Loc) -> None:
        raise NotImplementedError("GraphDataStorage is read only storage.")

    def is_node_finished(self, node_location: Loc) -> bool:
        return False

    def link_outputs(
        self,
        new_location: Loc,
        new_port: PortID,
        old_location: Loc,
        old_port: PortID,
    ) -> None:
        raise NotImplementedError("GraphDataStorage is read only storage.")

    def write_output(
        self, node_location: Loc, output_name: PortID, value: bytes
    ) -> Path:
        raise NotImplementedError("GraphDataStorage is read only storage.")

    def read_output(self, node_location: Loc, output_name: PortID) -> bytes:
        node, graph = graph_node_from_loc(node_location, self.graph)
        if -1 == get_last_index(node_location) and output_name == "body":
            return graph.model_dump_json().encode()

        outputs = _build_node_outputs(node)
        if output_name in outputs:
            if output := outputs[output_name]:
                return output
            return b"null"
        raise TierkreisError(f"No output named {output_name} in node {node_location}")

    def read_output_ports(self, node_location: Loc) -> list[PortID]:
        node, _ = graph_node_from_loc(node_location, self.graph)
        outputs = _build_node_outputs(node)
        return list(filter(lambda k: k != "*", outputs.keys()))

    def is_node_started(self, node_location: Loc) -> bool:
        return False

    def read_metadata(self, node_location: Loc) -> dict[str, Any]:
        return self.nodes[node_location].metadata

    def write_metadata(self, node_location: Loc) -> None:
        raise NotImplementedError("GraphDataStorage is read only storage.")

    def read_started_time(self, node_location: Loc) -> str | None:
        return None

    def read_finished_time(self, node_location: Loc) -> str | None:
        return None


def _build_node_outputs(node: NodeDef) -> dict[PortID, None | bytes]:
    outputs: dict[PortID, None | bytes] = {val: None for val in node.outputs}
    if "*" in outputs:
        outputs["0"] = None
    return outputs
