from pathlib import Path
from uuid import UUID
from typing import Any
from pydantic import BaseModel, Field

from tierkreis import Labels
from tierkreis.controller.data.core import PortID
from tierkreis.controller.data.graph import (
    GraphData,
    NodeDef,
    graph_node_from_loc,
)
from tierkreis.controller.data.types import bytes_from_ptype
from tierkreis.controller.data.location import Loc, OutputLoc, WorkerCallArgs
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

    def write_graph_def(self, node_location: Loc, graph: OutputLoc) -> None:
        raise NotImplementedError("GraphDataStorage is read only storage.")

    def read_node_def(self, node_location: Loc) -> NodeDef:
        match node_location.pop_last()[0]:
            case ("N", _):
                node, _ = graph_node_from_loc(node_location, self.graph)
                assert node is not None
                return node
            case _:
                raise TierkreisError(
                    f"Node location {node_location} is not a valid node location."
                )

    def read_graph_def(self, node_location: Loc) -> GraphData:
        match node_location.pop_last()[0]:
            case ("M", _) | ("L", _) | "-":
                _, graph = graph_node_from_loc(node_location, self.graph)
                return graph
            case _:
                raise TierkreisError(
                    f"Node location {node_location} is not a valid graph"
                )

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
        assert node_location.peek_index() >= 0
        node, graph = graph_node_from_loc(node_location, self.graph)

        output_ports = node.outputs.keys() if node else graph.output_ports
        if output_name not in output_ports:
            raise TierkreisError(
                f"No output named {output_name} in node {node_location}"
            )
        if node is not None and node.type == "const" and output_name == Labels.VALUE:
            return bytes_from_ptype(node.value)
        return b"null"

    def read_output_ports(self, node_location: Loc) -> list[PortID]:
        node, graph = graph_node_from_loc(node_location, self.graph)
        if node is None:
            return graph.output_ports
        return list(filter(lambda k: k != "*", node.outputs))

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
