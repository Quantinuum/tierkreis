from glob import glob
import os
import shutil
from pathlib import Path
from time import time_ns
from uuid import UUID

from tierkreis.controller.storage.protocol import (
    StorageEntryMetadata,
    ControllerStorage,
)


class ControllerFileStorage(ControllerStorage):
    def __init__(
        self,
        workflow_id: UUID,
        name: str | None = None,
        tierkreis_directory: Path = Path.home() / ".tierkreis" / "checkpoints",
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
        return [Path(x) for x in glob(f"{path}*/*")]

    def link(self, src: Path, dst: Path) -> None:
        dst.parent.mkdir(parents=True, exist_ok=True)
        os.link(src, dst)

    def mkdir(self, path: Path) -> None:
        return path.mkdir(parents=True, exist_ok=True)

    def read(self, path: Path) -> bytes:
        with open(path, "rb") as fh:
            return fh.read()

    def touch(self, path: Path, is_dir: bool = False) -> None:
        if is_dir:
            path.mkdir(parents=True, exist_ok=True)
            return

        path.parent.mkdir(parents=True, exist_ok=True)
        path.touch()

    def stat(self, path: Path) -> StorageEntryMetadata:
        return StorageEntryMetadata(path.stat().st_mtime)

    def write(self, path: Path, value: bytes) -> None:
        path.parent.mkdir(parents=True, exist_ok=True)
        with open(path, "wb+") as fh:
            fh.write(value)
