import os
import shutil
from pathlib import Path
from time import time_ns
from uuid import UUID

from tierkreis.controller.storage.exceptions import EntryNotFound
from tierkreis.controller.storage.protocol import (
    ControllerStorage,
    StorageEntryMetadata,
)

DEFAULT_DIRECTORY = Path.home() / ".tierkreis" / "checkpoints"


class ControllerFileStorage(ControllerStorage):
    def __init__(
        self,
        workflow_id: UUID,
        name: str | None = None,
        tierkreis_directory: Path = DEFAULT_DIRECTORY,
        *,
        do_cleanup: bool = False,
    ) -> None:
        self.tkr_dir = tierkreis_directory
        self.workflow_id = workflow_id
        self.name = name
        if do_cleanup:
            self.delete(self.workflow_dir)

    def delete(self, path: Path) -> None:
        uid = os.getuid()
        tmp_dir = Path(f"/tmp/{uid}/tierkreis/archive/{self.workflow_id}/{time_ns()}")
        tmp_dir.mkdir(parents=True, exist_ok=True)
        if self.exists(path):
            shutil.move(path, tmp_dir)

    def exists(self, path: Path) -> bool:
        return path.exists()

    def list_subpaths(self, path: Path) -> list[Path]:
        return list(path.iterdir())

    def link(self, src: Path, dst: Path) -> None:
        dst.parent.mkdir(parents=True, exist_ok=True)
        if dst.exists() and dst.resolve() == src:
            return  # We have already linked correctly

        try:
            os.link(src, dst)
        except (FileNotFoundError, FileExistsError) as exc:
            raise EntryNotFound(src) from exc

    def mkdir(self, path: Path) -> None:
        return path.mkdir(parents=True, exist_ok=True)

    def read(self, path: Path) -> bytes:
        try:
            with Path.open(path, "rb") as fh:
                return fh.read()
        except FileNotFoundError as exc:
            raise EntryNotFound(path) from exc

    def touch(self, path: Path, *, is_dir: bool = False) -> None:
        if is_dir:
            path.mkdir(parents=True, exist_ok=True)
            return

        path.parent.mkdir(parents=True, exist_ok=True)
        path.touch()

    def stat(self, path: Path) -> StorageEntryMetadata:
        return StorageEntryMetadata(path.stat().st_mtime)

    def write(self, path: Path, value: bytes) -> None:
        path.parent.mkdir(parents=True, exist_ok=True)
        with Path.open(path, "wb+") as fh:
            fh.write(value)
