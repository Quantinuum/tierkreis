"""In memory implementation of a storage layer."""

from pathlib import Path
from time import time
from typing import override
from uuid import UUID

from tierkreis.controller.storage.exceptions import EntryNotFound
from tierkreis.controller.storage.protocol import (
    ControllerStorage,
    StorageEntryMetadata,
)


class InMemoryFileData:
    value: bytes
    stats: StorageEntryMetadata

    def __init__(self, value: bytes) -> None:
        self.value = value
        self.stats = StorageEntryMetadata(time())


class ControllerInMemoryStorage(ControllerStorage):
    """In-memory implementation of ControllerStorage.

    Stores workflow files in memory using a dictionary instead of the filesystem.
    Uses a mapping Path -> FileData to emulate the required filesystem structure.
    Useful when debugging applications in conjunction with the InMemoryExecutor.
    """

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

    @override
    def delete(self, path: Path) -> None:
        self.files = {}

    @override
    def exists(self, path: Path) -> bool:
        return path in list(self.files.keys())

    @override
    def list_subpaths(self, path: Path) -> list[Path]:
        if path == self.workflow_dir:
            nodes = {

                    Path("/".join(str(x).split("/")[:2]))
                    for x in self.files
                    if str(x).startswith(str(path) + "/")
                }
            return list(nodes)
        return [x for x in self.files if str(x).startswith(str(path) + "/")]

    @override
    def link(self, src: Path, dst: Path) -> None:
        try:
            self.files[dst] = self.files[src]
        except KeyError as exc:
            raise EntryNotFound(src) from exc

    @override
    def mkdir(self, path: Path) -> None:
        return

    @override
    def read(self, path: Path) -> bytes:
        try:
            return self.files[path].value
        except KeyError as exc:
            raise EntryNotFound(path) from exc

    @override
    def touch(self, path: Path) -> None:
        self.files[path] = InMemoryFileData(b"")

    @override
    def stat(self, path: Path) -> StorageEntryMetadata:
        return self.files[path].stats

    @override
    def write(self, path: Path, value: bytes) -> None:
        self.files[path] = InMemoryFileData(value)
