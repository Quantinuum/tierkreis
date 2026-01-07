from pathlib import Path

from tierkreis.controller.executor.protocol import ControllerExecutor
from tierkreis.exceptions import TierkreisError


class MultipleExecutor:
    """Composes multiple executors into a single object.

    Implements: :py:class:`tierkreis.controller.executor.protocol.ControllerExecutor`
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
        enable_logging: bool = True,
    ) -> None:
        executor_name = self.assignments.get(launcher_name, None)
        # If there is no assignment for the worker, use the default.
        if executor_name is None:
            return self.default.run(
                launcher_name, worker_call_args_path, enable_logging
            )
        executor = self.executors.get(executor_name)
        if executor is None:
            raise TierkreisError(
                f"{launcher_name} is assigned to non-existent executor name: {executor_name}."
            )

        return executor.run(launcher_name, worker_call_args_path, enable_logging)
