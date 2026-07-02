"""Default file storage implementation."""

import os
import shutil
import logging
from pathlib import Path
from time import time_ns
from typing import override
from uuid import UUID

from tierkreis.controller.storage.exceptions import EntryNotFoundError
from tierkreis.controller.storage.protocol import (
    ControllerStorage,
    StorageEntryMetadata,
)

DEFAULT_DIRECTORY = Path.home() / ".tierkreis" / "checkpoints"


class ControllerFileStorage(ControllerStorage):
    """Storage backend using the filesystem.

    This storage implementation operates by relegating calls to the os filesystem.
    Calling with `do_cleanup` will ensure that previous runs are deleted.
    """

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

    @override
    def delete(self, path: Path) -> None:
        uid = os.getuid()
        tmp_dir = Path(f"/tmp/{uid}/tierkreis/archive/{self.workflow_id}/{time_ns()}")
        tmp_dir.mkdir(parents=True, exist_ok=True)
        if self.exists(path):
            try:
                shutil.move(path, tmp_dir)
            except OSError as e:
                logging.warning(
                    "Failed to move %s to %s for deletion: %s", path, tmp_dir, e
                )
                shutil.rmtree(path, ignore_errors=True)

    @override
    def exists(self, path: Path) -> bool:
        return path.exists()

    @override
    def list_subpaths(self, path: Path) -> list[Path]:
        return list(path.iterdir())

    @override
    def link(self, src: Path, dst: Path) -> None:
        dst.parent.mkdir(parents=True, exist_ok=True)
        if dst.exists() and dst.resolve() == src:
            return  # We have already linked correctly

        try:
            os.link(src, dst)
        except (FileNotFoundError, FileExistsError) as exc:
            raise EntryNotFoundError(src) from exc

    @override
    def mkdir(self, path: Path) -> None:
        return path.mkdir(parents=True, exist_ok=True)

    @override
    def read(self, path: Path) -> bytes:
        try:
            with Path.open(path, "rb") as fh:
                return fh.read()
        except FileNotFoundError as exc:
            raise EntryNotFoundError(path) from exc

    @override
    def touch(self, path: Path, *, is_dir: bool = False) -> None:
        if is_dir:
            path.mkdir(parents=True, exist_ok=True)
            return

        path.parent.mkdir(parents=True, exist_ok=True)
        path.touch()

    @override
    def stat(self, path: Path) -> StorageEntryMetadata:
        return StorageEntryMetadata(path.stat().st_mtime)

    @override
    def write(self, path: Path, value: bytes) -> None:
        path.parent.mkdir(parents=True, exist_ok=True)
        with Path.open(path, "wb+") as fh:
            fh.write(value)
