import logging
import os
import shutil
import subprocess
from pathlib import Path

from tierkreis.consts import TKR_DIR_KEY
from tierkreis.exceptions import TierkreisError

logger = logging.getLogger(__name__)


class UvExecutor:
    """Executes workers in an UV python environment.

    Implements: :py:class:`tierkreis.controller.executor.protocol.ControllerExecutor`
    """

    def __init__(
        self,
        registry_path: Path,
        logs_path: Path,
        env: dict[str, str] | None = None,
    ) -> None:
        self.launchers_path = registry_path
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

        worker_path = self.launchers_path / launcher_name

        env = os.environ.copy() | self.env.copy()
        if "VIRTUAL_ENVIRONMENT" not in env:
            env["VIRTUAL_ENVIRONMENT"] = ""
        if TKR_DIR_KEY not in env:
            env[TKR_DIR_KEY] = str(self.logs_path.parent.parent)
        _error_path = self.errors_path.parent / "_error"
        tee_str = f">(tee -a {self.errors_path!s} {self.logs_path!s} >/dev/null)"
        proc = subprocess.Popen(
            ["/bin/bash"],
            start_new_session=True,
            stdin=subprocess.PIPE,
            cwd=worker_path,
            env=env,
        )
        proc.communicate(
            f"({uv_path} run main.py {worker_call_args_path} > {tee_str} 2> {tee_str}"
            f" || touch {_error_path}) &".encode(),
            timeout=10,
        )
