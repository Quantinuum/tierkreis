from pathlib import Path
from typing import Protocol


class ControllerExecutor(Protocol):
    """The executor protocol defines how to run workers.

    An executor is responsible for running a worker(binary) in a given environment.

    """

    def run(
        self,
        launcher_name: str,
        worker_call_args_path: Path,
        enable_logging: bool = True,
    ) -> None:
        """Run the node defined by the node_definition path.

        Specifies the worker to run by its launcher name.
        For example the function "builtins.iadd" will call the builtins worker's iadd function.
        The call arguments for the function call are retrieved retrieved from its location.

        :param launcher_name: module description of launcher to run.
        :type launcher_name: str
        :param worker_call_args_path: Location of the worker call args.
        :type worker_call_args_path: Path
        """

    ...
