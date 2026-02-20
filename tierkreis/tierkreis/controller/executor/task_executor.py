import json
from fnmatch import filter
from pathlib import Path

from tierkreis.controller.data.location import WorkerCallArgs
from tierkreis.controller.executor.protocol import ControllerExecutor
from tierkreis.controller.storage.protocol import ControllerStorage
from tierkreis.exceptions import TierkreisError


class TaskExecutor:
    """A Tierkreis executor that routes tasks to other executors based on the fully qualified task name.

    The fully qualified task name is of the form <WORKER_NAME>.<TASK_NAME> .
    Glob syntax can be used to route multiple tasks to the same executor.
    """

    def __init__(
        self,
        assignments: dict[str, ControllerExecutor],
        storage: ControllerStorage,
    ) -> None:
        self.assignments = assignments
        self.workflow_dir = storage.workflow_dir

    def run(self, launcher_name: str, worker_call_args_path: Path) -> None:
        with open(self.workflow_dir.parent / worker_call_args_path) as fh:
            call_args = WorkerCallArgs(**json.load(fh))

        qualified_task = f"{launcher_name}.{call_args.function_name}"
        for pattern, executor in self.assignments.items():
            matching = filter([qualified_task], pattern)
            if matching:
                executor.run(launcher_name, worker_call_args_path)
                return

        msg = f"No assigned executor for task {qualified_task}"
        raise TierkreisError(msg)
