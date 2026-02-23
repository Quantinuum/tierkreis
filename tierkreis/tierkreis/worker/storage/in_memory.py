"""In-memory storage implementation analog to ControllerInMemoryStorage."""

# ruff: noqa: D102 (class methods inherited from WorkerStorage)
import fnmatch
import json
import logging
from pathlib import Path

from tierkreis.controller.data.location import WorkerCallArgs
from tierkreis.controller.storage.exceptions import EntryNotFoundError
from tierkreis.controller.storage.in_memory import (
    ControllerInMemoryStorage,
    InMemoryFileData,
)
from tierkreis.exceptions import TierkreisError

logger = logging.getLogger(__name__)


class InMemoryWorkerStorage:
    """In-memory storage implementation for workers.

    Delegates calls to the ControllerInMemoryStorage used for the workflow.
    :fields:
        controller_storage: The controller storage.
    """

    def __init__(self, controller_storage: ControllerInMemoryStorage) -> None:
        self.controller_storage = controller_storage

    def resolve(self, path: Path | str) -> Path:
        return Path(path)

    def read_call_args(self, path: Path) -> WorkerCallArgs:
        try:
            bs = self.controller_storage.files[path].value
            return WorkerCallArgs(**json.loads(bs))
        except KeyError as exc:
            raise EntryNotFoundError(path) from exc

    def read_input(self, path: Path) -> bytes:
        try:
            return self.controller_storage.files[path].value
        except KeyError as exc:
            raise EntryNotFoundError(path) from exc

    def write_output(self, path: Path, value: bytes) -> None:
        self.controller_storage.files[path] = InMemoryFileData(value)

    def glob(self, path_string: str) -> list[str]:
        files = [str(x) for x in self.controller_storage.files]
        return fnmatch.filter(files, path_string)

    def mark_done(self, path: Path) -> None:
        self.controller_storage.touch(path)

    def write_error(self, _: Path, error_logs: str) -> None:
        logger.error(error_logs)
        msg = "Error occurred when running graph in-memory."
        raise TierkreisError(msg)
