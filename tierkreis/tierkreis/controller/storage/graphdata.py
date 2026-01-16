# ruff: noqa: ARG002 (methods to fulfill interface contract but not possible in storage)
import json
from pathlib import Path
from typing import Any
from uuid import UUID

from pydantic import BaseModel, Field

from tierkreis.controller.data.core import PortID
from tierkreis.controller.data.graph import (
    Eval,
    GraphData,
    NodeDef,
    graph_node_from_loc,
)
from tierkreis.controller.data.location import Loc, OutputLoc, WorkerCallArgs
from tierkreis.controller.storage.protocol import (
    ControllerStorage,
    StorageEntryMetadata,
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
        msg = "GraphDataStorage is read only storage."
        raise NotImplementedError(msg)

    def exists(self, path: Path) -> bool:
        msg = "GraphDataStorage is only for graph construction."
        raise NotImplementedError(msg)

    def list_subpaths(self, path: Path) -> list[Path]:
        msg = "GraphDataStorage uses GraphData not paths."
        raise NotImplementedError(msg)

    def link(self, src: Path, dst: Path) -> None:
        msg = "GraphDataStorage is read only storage."
        raise NotImplementedError(msg)

    def mkdir(self, path: Path) -> None:
        msg = "GraphDataStorage is read only storage."
        raise NotImplementedError(msg)

    def read(self, path: Path) -> bytes:
        msg = "GraphDataStorage uses GraphData not paths."
        raise NotImplementedError(msg)

    def stat(self, path: Path) -> StorageEntryMetadata:
        msg = "GraphDataStorage is only for graph construction."
        raise NotImplementedError(msg)

    def touch(self, path: Path) -> None:
        msg = "GraphDataStorage is read only storage."
        raise NotImplementedError(msg)

    def write(self, path: Path, value: bytes) -> None:
        msg = "GraphDataStorage is read only storage."
        raise NotImplementedError(msg)

    def write_node_def(self, node_location: Loc, node: NodeDef) -> None:
        msg = "GraphDataStorage is read only storage."
        raise NotImplementedError(msg)

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
        msg = "GraphDataStorage is read only storage."
        raise NotImplementedError(msg)

    def read_worker_call_args(self, node_location: Loc) -> WorkerCallArgs:
        msg = f"Node location {node_location} doesn't have a associate call args."
        raise TierkreisError(
            msg,
        )

    def read_errors(self, node_location: Loc = Loc()) -> str:
        return ""

    def node_has_error(self, node_location: Loc) -> bool:
        return False

    def write_node_errors(self, node_location: Loc, error_logs: str) -> None:
        msg = "GraphDataStorage is read only storage."
        raise NotImplementedError(msg)

    def mark_node_finished(self, node_location: Loc) -> None:
        msg = "GraphDataStorage is read only storage."
        raise NotImplementedError(msg)

    def is_node_finished(self, node_location: Loc) -> bool:
        return False

    def link_outputs(
        self,
        new_location: Loc,
        new_port: PortID,
        old_location: Loc,
        old_port: PortID,
    ) -> None:
        msg = "GraphDataStorage is read only storage."
        raise NotImplementedError(msg)

    def write_output(
        self,
        node_location: Loc,
        output_name: PortID,
        value: bytes,
    ) -> Path:
        msg = "GraphDataStorage is read only storage."
        raise NotImplementedError(msg)

    def read_output(self, node_location: Loc, output_name: PortID) -> bytes:
        node, graph = graph_node_from_loc(node_location, self.graph)
        if node_location.peek_index() == -1 and output_name == "body":
            return graph.model_dump_json().encode()

        outputs = _build_node_outputs(node)
        if output_name in outputs:
            if output := outputs[output_name]:
                return output
            return b"null"
        msg = f"No output named {output_name} in node {node_location}"
        raise TierkreisError(msg)

    def read_output_ports(self, node_location: Loc) -> list[PortID]:
        node, _ = graph_node_from_loc(node_location, self.graph)
        outputs = _build_node_outputs(node)
        return list(filter(lambda k: k != "*", outputs.keys()))

    def is_node_started(self, node_location: Loc) -> bool:
        return False

    def read_metadata(self, node_location: Loc) -> dict[str, Any]:
        return self.nodes[node_location].metadata

    def write_metadata(self, node_location: Loc) -> None:
        msg = "GraphDataStorage is read only storage."
        raise NotImplementedError(msg)

    def read_started_time(self, node_location: Loc) -> str | None:
        return None

    def read_finished_time(self, node_location: Loc) -> str | None:
        return None


def _build_node_outputs(node: NodeDef) -> dict[PortID, None | bytes]:
    if node.type == "const":
        if isinstance(node.value, dict):
            if "nodes" not in node.value:
                return {"value": json.dumps(node.value).encode()}
            return {"value": b"Graph"}
        if isinstance(node.value, GraphData):
            return {"value": b"Graph"}
        return {"value": json.dumps(node.value).encode()}
    outputs: dict[PortID, None | bytes] = dict.fromkeys(node.outputs)
    if "*" in outputs:
        outputs["0"] = None
    return outputs
