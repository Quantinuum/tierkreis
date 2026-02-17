from dataclasses import dataclass, field
from pathlib import Path
from uuid import UUID
from time import time

from tierkreis.controller.storage.exceptions import EntryNotFound
from tierkreis.controller.storage.protocol import (
    StorageEntryMetadata,
    ControllerStorage,
)


@dataclass
class InMemoryFileData:
    value: bytes
    # stats: StorageEntryMetadata

    def __post_init__(self) -> None:
        self.stats = StorageEntryMetadata(time())


@dataclass
class ControllerInMemoryStorage(ControllerStorage):
    workflow_id: UUID
    name: str | None = None
    tkr_dir: Path = Path()
    files: dict[Path, InMemoryFileData] = field(default_factory=lambda: {})

    def delete(self, path: Path) -> None:
        self.files = {}

    def exists(self, path: Path) -> bool:
        return path in list(self.files.keys())

    def list_subpaths(self, path: Path) -> list[Path]:
        if path == self.workflow_dir:
            nodes = set(
                [
                    Path("/".join(str(x).split("/")[:2]))
                    for x in self.files.keys()
                    if str(x).startswith(str(path) + "/")
                ]
            )
            return list(nodes)
        return [x for x in self.files.keys() if str(x).startswith(str(path) + "/")]

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
