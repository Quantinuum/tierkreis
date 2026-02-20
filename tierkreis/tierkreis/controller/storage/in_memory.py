from pathlib import Path
from time import time
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

    def delete(self, path: Path) -> None:
        self.files = {}

    def exists(self, path: Path) -> bool:
        return path in list(self.files.keys())

    def list_subpaths(self, path: Path) -> list[Path]:
        if path == self.workflow_dir:
            nodes = {

                    Path("/".join(str(x).split("/")[:2]))
                    for x in self.files
                    if str(x).startswith(str(path) + "/")
                }
            return list(nodes)
        return [x for x in self.files if str(x).startswith(str(path) + "/")]

    def link(self, src: Path, dst: Path) -> None:
        try:
            self.files[dst] = self.files[src]
        except KeyError as exc:
            raise EntryNotFound(src) from exc

    def mkdir(self, path: Path) -> None:
        return

    def read(self, path: Path) -> bytes:
        try:
            return self.files[path].value
        except KeyError as exc:
            raise EntryNotFound(path) from exc

    def touch(self, path: Path, is_dir: bool = False) -> None:
        self.files[path] = InMemoryFileData(b"")

    def stat(self, path: Path) -> StorageEntryMetadata:
        return self.files[path].stats

    def write(self, path: Path, value: bytes) -> None:
        self.files[path] = InMemoryFileData(value)
