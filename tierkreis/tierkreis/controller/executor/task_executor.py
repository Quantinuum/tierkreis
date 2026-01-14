import json
from pathlib import Path
from fnmatch import filter

from tierkreis.controller.data.location import WorkerCallArgs
from tierkreis.controller.executor.protocol import ControllerExecutor


class TaskExecutor:
    def __init__(self, assignments: dict[str, ControllerExecutor]) -> None:
        self.assignments = assignments

    def run(self, launcher_name: str, worker_call_args_path: Path) -> None:
        with open(worker_call_args_path) as fh:
            call_args = WorkerCallArgs(**json.load(fh))

        qualified_task = f"{launcher_name}.{call_args.function_name}"
        for pattern, executor in self.assignments.items():
            matching = filter([qualified_task], pattern)
            if matching:
                executor.run(launcher_name, worker_call_args_path)
                return
