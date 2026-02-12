import logging
import os
import shutil
import subprocess
from pathlib import Path

from tierkreis.consts import TKR_DIR_KEY
from tierkreis.controller.executor.check_launcher import check_and_set_launcher
from tierkreis.controller.executor.registries import find_registry_for_worker
from tierkreis.controller.storage.data import ExecutorData
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
    ) -> ExecutorData:
        self.errors_path = (
            self.logs_path.parent.parent
            / worker_call_args_path.parent
            / "logs"  # made we should change this
        )
        logger.info("START %s %s", launcher_name, worker_call_args_path)

        if uv_path is None:
            uv_path = shutil.which("uv")
        if uv_path is None:
            raise TierkreisError("uv is required to use the uv_executor")

        registry_path = find_registry_for_worker(launcher_name, self.registries)
        worker_path = check_and_set_launcher(registry_path, launcher_name, ".py").parent
        env = os.environ.copy() | self.env.copy()
        if "VIRTUAL_ENVIRONMENT" not in env:
            env["VIRTUAL_ENVIRONMENT"] = ""
        if TKR_DIR_KEY not in env:
            env[TKR_DIR_KEY] = str(self.logs_path.parent.parent)
        _error_path = self.errors_path.parent / "_error"
        tee_str = f">(tee -a {str(self.errors_path)} {str(self.logs_path)} >/dev/null)"
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
            command, env, registry_path / launcher_name, uv_path
        )  # TODO update this when docs are merged

    def _generate_debug_data(
        self, command: str, env: dict[str, str], cwd: Path, uv_path: str
    ) -> ExecutorData:
        launcher_command = f"cd {cwd} && {command}"
        return ExecutorData(
            str(__class__), launcher_command, env=env, packages=_uv_freeze(uv_path, cwd)
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
