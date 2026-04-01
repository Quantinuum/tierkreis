"""In memory implementation."""

# ruff: noqa: D102 (class methods inherited from ControllerExecutor)
from importlib.machinery import ModuleSpec
import importlib.util
import json
import logging
from pathlib import Path

from tierkreis.controller.data.location import WorkerCallArgs
from tierkreis.controller.executor.check_launcher import check_and_set_launcher
from tierkreis.controller.storage.data import ExecutorDebugData
from tierkreis.controller.storage.in_memory import ControllerInMemoryStorage
from tierkreis.exceptions import TierkreisError
from tierkreis.worker.storage.in_memory import InMemoryWorkerStorage

logger = logging.getLogger(__name__)


class InMemoryExecutor:
    """Execute workers in the same process as the controller.

    Loads the worker as python module if possible.
    Cannot only run python workers in conjunction with ControllerInMemoryStorage.
    Implements: :py:class:`tierkreis.controller.executor.protocol.ControllerExecutor`

    :fields:
        registry_path (Path): The locations to search for worker modules.
        storage (ControllerInMemoryStorage):
            Storage reference to access in memory values.
    """

    def __init__(self, registry_path: Path, storage: ControllerInMemoryStorage) -> None:
        self.registry_path = registry_path
        self.storage = storage

    def run(
        self,
        launcher_name: str,
        worker_call_args_path: Path,
    ) -> ExecutorDebugData:
        logger.info("START %s %s", launcher_name, worker_call_args_path)
        call_args = WorkerCallArgs(
            **json.loads(self.storage.read(worker_call_args_path)),
        )
        launcher_path = check_and_set_launcher(
            self.registry_path, launcher_name, ".py", check_shell=True
        )
        spec = _find_spec(launcher_path)
        if spec is None or spec.loader is None:
            msg = (
                f"Couldn't load module main.py in {self.registry_path / launcher_name}"
            )
            raise TierkreisError(
                msg,
            )
        module = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(module)
        worker_storage = InMemoryWorkerStorage(self.storage)
        module.worker.storage = worker_storage
        module.worker.functions[call_args.function_name](call_args)
        self.storage.touch(call_args.done_path)
        return self._generate_debug_data(launcher_path)

    def _generate_debug_data(self, launcher_path: Path) -> ExecutorDebugData:
        return ExecutorDebugData(
            executor=str(__class__),
            launch_command=str(launcher_path),
        )


def _find_spec(launcher_path: Path) -> ModuleSpec | None:
    if launcher_path.suffix == ".py":
        return importlib.util.spec_from_file_location("in_memory", launcher_path)
    launcher = launcher_path.name
    if not launcher.startswith("tkr"):
        launcher = "tkr-" + launcher

    return importlib.util.find_spec((launcher + "_impl").replace("-", "_"))
