import logging
import os
import shutil
import subprocess
from pathlib import Path

from tierkreis.consts import TKR_DIR_KEY
from tierkreis.controller.executor.check_launcher import check_and_set_launcher
from tierkreis.controller.executor.registries import find_registry_for_worker
from tierkreis.exceptions import TierkreisError

logger = logging.getLogger(__name__)


class UvExecutor:
    """Executes workers in an UV python environment.

    Implements: :py:class:`tierkreis.controller.executor.protocol.ControllerExecutor`
    """

    def __init__(
        self,
        registry_path: Path | list[Path],
        logs_path: Path,
        env: dict[str, str] | None = None,
    ) -> None:
        self.registries = registry_path
        self.logs_path = logs_path
        self.errors_path = logs_path
        self.env = env or {}

    def run(
        self,
        launcher_name: str,
        worker_call_args_path: Path,
        uv_path: str | None = None,
    ) -> None:
        self.errors_path = (
            self.logs_path.parent.parent
            / worker_call_args_path.parent
            / "logs"  # made we should change this
        )
        logger.info("START %s %s", launcher_name, worker_call_args_path)

        if uv_path is None:
            uv_path = shutil.which("uv")
        if uv_path is None:
            msg = "uv is required to use the uv_executor"
            raise TierkreisError(msg)

        registry_path = find_registry_for_worker(launcher_name, self.registries)
        worker_path = check_and_set_launcher(registry_path, launcher_name, ".py").parent
        env = os.environ.copy() | self.env.copy()
        if "VIRTUAL_ENVIRONMENT" not in env:
            env["VIRTUAL_ENVIRONMENT"] = ""
        if TKR_DIR_KEY not in env:
            env[TKR_DIR_KEY] = str(self.logs_path.parent.parent)
        _error_path = self.errors_path.parent / "_error"
        tee_str = f">(tee -a {self.errors_path!s} {self.logs_path!s} >/dev/null)"
        proc = subprocess.Popen(
            ["bash"],
            start_new_session=True,
            stdin=subprocess.PIPE,
            cwd=worker_path,
            env=env,
        )
        proc.communicate(
            f"({uv_path} run main.py {worker_call_args_path} > {tee_str} 2> {tee_str} || touch {_error_path}) &".encode(),
            timeout=10,
        )
