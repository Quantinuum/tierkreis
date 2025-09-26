from pathlib import Path
from uuid import UUID
from typing import Any
from time import time

from pydantic import BaseModel, Field

from tierkreis.controller.data.core import PortID
from tierkreis.controller.data.graph import NodeDef
from tierkreis.controller.data.location import WorkerCallArgs

from tierkreis.controller.storage.pathstorage import PathStorageBase, StatResult


class InMemoryFileData:
    value: bytes
    stats: StatResult

    def __init__(self, value: bytes) -> None:
        self.value = value
        self.stats = StatResult(time())


class InMemoryBackend:
    def __init__(
        self,
        workflow_id: UUID,
        name: str | None = None,
        tierkreis_directory: Path = Path(),
    ) -> None:
        self.tkr_dir = tierkreis_directory
        self.workflow_id = workflow_id
        self.name = name

        self.files: dict[Path, InMemoryFileData] = {}

    def delete(self) -> None:
        self.files = {}

    def exists(self, path: Path) -> bool:
        return path in list(self.files.keys())

    def list_output_paths(self, output_dir: Path) -> list[Path]:
        return [
            x for x in self.files.keys() if str(x).startswith(str(output_dir) + "/")
        ]

    def link(self, src: Path, dst: Path) -> None:
        self.files[dst] = self.files[src]

    def read(self, path: Path) -> bytes:
        return self.files[path].value

    def touch(self, path: Path, is_dir: bool = False) -> None:
        self.files[path] = InMemoryFileData(b"")

    def stat(self, path: Path) -> StatResult:
        return self.files[path].stats

    def write(self, path: Path, value: bytes) -> None:
        self.files[path] = InMemoryFileData(value)


class ControllerInMemoryStorage(InMemoryBackend, PathStorageBase): ...


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
