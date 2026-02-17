import importlib.util
import logging
from dataclasses import dataclass
from pathlib import Path
from typing import Any

from tierkreis.consts import PACKAGE_PATH
from tierkreis.controller.executor.check_launcher import check_and_set_launcher
from tierkreis.exceptions import TierkreisError

logger = logging.getLogger(__name__)


@dataclass
class InMemoryExecutor:
    """Executes workers in the same process as the controller.

    Implements: :py:class:`tierkreis.controller.executor.protocol.ControllerExecutor`
    """

    registry_path: Path

    def run(
        self,
        launcher_name: str,
        worker_call_args_path: Path,
        storage: Any | None = None,
    ) -> None:
        from tierkreis.controller.storage.in_memory import ControllerInMemoryStorage
        from tierkreis.worker.storage.in_memory import (
            InMemoryWorkerStorage,
        )  # to avoid circuilar dependency

        if storage is None or not isinstance(storage, ControllerInMemoryStorage):
            msg = "Storage in InMemoryExecutor must be ControllerInMemoryStorage!"
            raise TierkreisError(msg)
        worker_storage = InMemoryWorkerStorage(storage)
        logger.info("START %s %s", launcher_name, worker_call_args_path)
        call_args = worker_storage.read_call_args(worker_call_args_path)
        if launcher_name == "builtins":
            registry_path = PACKAGE_PATH / "tierkreis"
        else:
            registry_path = self.registry_path

        launcher_path = check_and_set_launcher(registry_path, launcher_name, ".py")
        spec = importlib.util.spec_from_file_location("in_memory", launcher_path)
        if spec is None or spec.loader is None:
            raise TierkreisError(
                f"Couldn't load module main.py in {registry_path / launcher_name}"
            )
        module = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(module)
        module.worker.storage = worker_storage
        module.worker.functions[call_args.function_name](call_args)
        storage.touch(call_args.done_path)
