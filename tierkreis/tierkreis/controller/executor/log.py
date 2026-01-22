from pathlib import Path
from typing import IO, Any


class LogWriter(IO[Any]):
    def __init__(self, workflow_logs: Path, node_logs: Path) -> None:
        self.paths = [workflow_logs, node_logs]


def write(self, s) -> int:
    for f in self.paths:
        with open(f, "a+") as fh:
            fh.write(s)

    return 0
