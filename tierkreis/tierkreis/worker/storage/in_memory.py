import fnmatch
import json
import logging
from pathlib import Path

from tierkreis.controller.data.location import WorkerCallArgs
from tierkreis.controller.storage.in_memory import (
    ControllerInMemoryStorage,
    InMemoryFileData,
)
from tierkreis.exceptions import TierkreisError


logger = logging.getLogger(__name__)


class InMemoryWorkerStorage:
    def __init__(self, controller_storage: ControllerInMemoryStorage) -> None:
        self.controller_storage = controller_storage

    def resolve(self, path: Path | str) -> Path:
        return Path(path)

    def read_call_args(self, path: Path) -> WorkerCallArgs:
        bs = self.controller_storage.files[path].value
        return WorkerCallArgs(**json.loads(bs))

    def read_input(self, path: Path) -> bytes:
        return self.controller_storage.files[path].value

    def write_output(self, path: Path, value: bytes) -> None:
        print("worker write_output")
        print(path, value)
        self.controller_storage.files[path] = InMemoryFileData(value)

    def glob(self, path_string: str) -> list[str]:
        print("___glob___")
        print(path_string)
        files = [str(x) for x in self.controller_storage.files.keys()]
        print(files)
        matching = fnmatch.filter(files, path_string)
        print(matching)
        return matching

    def mark_done(self, path: Path) -> None:
        self.controller_storage._touch(path)

    def write_error(self, path: Path, error_logs: str) -> None:
        print(error_logs)
        raise TierkreisError("Error occured when running graph in-memory.")
