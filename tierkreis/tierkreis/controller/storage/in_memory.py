from pathlib import Path
from uuid import UUID
from time import time


from tierkreis.controller.storage.base import StatResult, TKRStorage


class InMemoryFileData:
    value: bytes
    stats: StatResult

    def __init__(self, value: bytes) -> None:
        self.value = value
        self.stats = StatResult(time())


class ControllerInMemoryStorage(TKRStorage):
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
