"""Default python executor based on uv."""

import logging
import os
import shutil
import subprocess
from pathlib import Path

from tierkreis.consts import TKR_DIR_KEY
from tierkreis.controller.executor.check_launcher import check_and_set_launcher
from tierkreis.controller.executor.registries import find_registry_for_worker
from tierkreis.controller.storage.data import ExecutorDebugData
from tierkreis.exceptions import TierkreisError

logger = logging.getLogger(__name__)


class UvExecutor:
    """Executes workers in an UV python environment.

    Depends on uv to run, hence the worker needs a pyproject.toml / a respective script.
    Works out of the box with the cli worker definitions.
    The env field can be used to provide additional variables; for example
    controlling the python / uv version through $VIRTUAL_ENVIRONMENT.
    Also to resolve paths, the $TKR_DIR will be set to the workflow directory.

    Implements: :py:class:`tierkreis.controller.executor.protocol.ControllerExecutor`

    :fields:
        launchers_path (Path): The locations to search for python workers.
        logs_path (Path): The controller log file.
        errors_path (Path): The controller error file for the function node.
        env: (dict[str,str]): Additional environments to hand to the spawned subprocess.
    """

    def __init__(
        self,
        registry_path: Path | list[Path],
        logs_path: Path,
        env: dict[str, str] | None = None,
        log_env_in_debug: bool = True,
    ) -> None:
        self.registries = registry_path
        self.logs_path = logs_path
        self.errors_path = logs_path
        self.env = env or {}
        self.log_env_in_debug = log_env_in_debug

    def run(
        self,
        launcher_name: str,
        worker_call_args_path: Path,
        uv_path: str | None = None,
    ) -> ExecutorDebugData:
        self.errors_path = (
            self.logs_path.parent.parent
            / worker_call_args_path.parent
            / "logs"  # maybe we should change this
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
        command = f"({uv_path} run main.py {worker_call_args_path} > {tee_str} 2> {tee_str} || touch {_error_path}) &"
        proc.communicate(
            command.encode(),
            timeout=10,
        )
        return self._generate_debug_data(
            command,
            env,
            registry_path / launcher_name,
            uv_path,
        )  # TODO update this when docs are merged

    def _generate_debug_data(
        self,
        command: str,
        env: dict[str, str],
        cwd: Path,
        uv_path: str,
    ) -> ExecutorDebugData:
        if not self.log_env_in_debug:
            env = {}
        launcher_command = f"cd {cwd} && {command}"
        return ExecutorDebugData(
            executor=str(__class__),
            launch_command=launcher_command,
            env=env,
            packages=_uv_freeze(uv_path, cwd),
        )


def _uv_freeze(uv_path: str, cwd: Path) -> list[str]:
    result = subprocess.run(
        [
            str(uv_path),
            "export",
            "--format",
            "requirements-txt",
            "--no-hashes",
            "--no-annotate",
        ],
        capture_output=True,
        text=True,
        check=True,
        cwd=cwd,
    )

    return result.stdout.splitlines()
