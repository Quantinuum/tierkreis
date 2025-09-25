import json
from glob import glob
import os
from pathlib import Path

from tierkreis.consts import TKR_DIR_KEY
from tierkreis.controller.data.location import WorkerCallArgs


class WorkerFileStorage:
    def __init__(self, tierkreis_dir: Path | None = None) -> None:
        if tierkreis_dir is not None:
            self.tierkreis_dir = tierkreis_dir
        elif tierkreis_dir_str := os.environ.get(TKR_DIR_KEY):
            self.tierkreis_dir = Path(tierkreis_dir_str).resolve()
        else:
            self.tierkreis_dir = Path.home() / ".tierkreis" / "checkpoints"

    def resolve(self, path: Path | str) -> Path:
        path = Path(path)
        return path if path.is_absolute() else self.tierkreis_dir / path

    def read_call_args(self, path: Path) -> WorkerCallArgs:
        with open(self.resolve(path), "r") as fh:
            return WorkerCallArgs(**json.loads(fh.read()))

    def read_input(self, path: Path) -> bytes:
        with open(self.resolve(path), "rb") as fh:
            return fh.read()

    def write_output(self, path: Path, value: bytes) -> None:
        with open(self.resolve(path), "wb+") as fh:
            fh.write(value)

    def glob(self, path_string: str) -> list[str]:
        return glob(str(self.resolve(path_string)))

    def mark_done(self, path: Path) -> None:
        self.resolve(path).touch()

    def write_error(self, path: Path, error_logs: str) -> None:
        with open(self.resolve(path), "w+") as f:
            f.write(error_logs)
