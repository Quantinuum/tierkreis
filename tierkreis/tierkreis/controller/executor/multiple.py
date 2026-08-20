"""A meta executor consisting of multiple single executors."""

from pathlib import Path

from tierkreis.controller.executor.protocol import ControllerExecutor
from tierkreis.controller.storage.data import ExecutorDebugData
from tierkreis.exceptions import TierkreisError


class MultipleExecutor:
    """Composes multiple executors into a single object.

    Will execute all worker tasks on the assigned executor or default.
    Implements: :py:class:`tierkreis.controller.executor.protocol.ControllerExecutor`

    :fields:
        default (ControllerExecutor): The default executor to use for all unspecified
            tasks.
        executors (dict[str, ControllerExecutor]): A mapping of name -> executor.
        assignments (dict[str, string]): A mapping of worker to executor name

    """

    def __init__(
        self,
        default: ControllerExecutor,
        executors: dict[str, ControllerExecutor],
        assignments: dict[str, str],
    ) -> None:
        self.default = default
        self.executors = executors
        self.assignments = assignments

    def run(
        self,
        launcher_name: str,
        worker_call_args_path: Path,
    ) -> ExecutorDebugData:
        executor_name = self.assignments.get(launcher_name, None)
        # If there is no assignment for the worker, use the default.
        if executor_name is None:
            data = self.default.run(launcher_name, worker_call_args_path)
            data.executor = f"{__class__}:" + data.executor
            return data
        executor = self.executors.get(executor_name)
        if executor is None:
            msg = (
                f"{launcher_name} is assigned to non-existent"
                f" executor name: {executor_name}."
            )
            raise TierkreisError(
                msg,
            )

        data = executor.run(launcher_name, worker_call_args_path)
        data.executor = f"{__class__}:" + data.executor
        return data
