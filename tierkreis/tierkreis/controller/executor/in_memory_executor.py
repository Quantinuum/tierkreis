import json
import logging
import importlib.util
from pathlib import Path
from typing import Callable

from tierkreis.controller.data.location import WorkerCallArgs
from tierkreis.controller.data.models import PModel
from tierkreis.controller.storage.in_memory import ControllerInMemoryStorage
from tierkreis.worker.storage.in_memory import InMemoryWorkerStorage
from tierkreis.exceptions import TierkreisError


logger = logging.getLogger(__name__)


class InMemoryExecutor:
    """Executes workers in the same process as the controller.

    Implements: :py:class:`tierkreis.controller.executor.protocol.ControllerExecutor`
    """

    def __init__(
        self,
        registry_path: Path,
        storage: ControllerInMemoryStorage,
        debug_values: dict[str, PModel | Callable[..., PModel]] | None = None,
    ) -> None:
        self.registry_path = registry_path
        self.storage = storage
        self.debug_values = debug_values or {}

    def run(
        self,
        launcher_name: str,
        worker_call_args_path: Path,
        debug: bool = False,
    ) -> None:
        logging.basicConfig(
            format="%(asctime)s: %(message)s",
            datefmt="%Y-%m-%dT%H:%M:%S%z",
            filemode="a",
            level=logging.INFO,
        )
        logger.info("START %s %s", launcher_name, worker_call_args_path)
        call_args = WorkerCallArgs(
            **json.loads(self.storage.read(worker_call_args_path))
        )

        spec = importlib.util.spec_from_file_location(
            "in_memory", self.registry_path / launcher_name / "main.py"
        )
        if spec is None or spec.loader is None:
            raise TierkreisError(
                f"Couldn't load main.py in {self.registry_path / launcher_name}"
            )
        module = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(module)
        worker_storage = InMemoryWorkerStorage(self.storage)
        module.worker.storage = worker_storage
        if debug:
            module.worker.run_debug(
                worker_call_args_path,
                self.debug_values.get(
                    f"{launcher_name}:{call_args.function_name}", None
                ),
            )
        else:
            module.worker.functions[call_args.function_name](call_args)

        self.storage.touch(call_args.done_path)
