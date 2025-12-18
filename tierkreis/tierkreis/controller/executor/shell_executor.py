import json
import os
import subprocess
from pathlib import Path

from tierkreis.consts import TKR_DIR_KEY
from tierkreis.controller.data.location import WorkerCallArgs
from tierkreis.exceptions import TierkreisError


class ShellExecutor:
    """Executes workers in an unix shell.

    Implements: :py:class:`tierkreis.controller.executor.protocol.ControllerExecutor`
    """

    def __init__(
        self,
        registry_path: Path,
        workflow_dir: Path,
        timeout: int = 10,
        env: dict[str, str] | None = None,
    ) -> None:
        self.launchers_path = registry_path
        self.logs_path = workflow_dir / "logs"
        self.errors_path = workflow_dir / "logs"
        self.workflow_dir = workflow_dir
        self.timeout = timeout
        self.env = env or {}

    def run(
        self,
        launcher_name: str,
        worker_call_args_path: Path,
        export_values: bool = False,
    ) -> None:
        launcher_path = self.launchers_path / launcher_name
        self.errors_path = worker_call_args_path.parent / "logs"

        if not launcher_path.exists():
            raise TierkreisError(f"Launcher not found: {launcher_name}.")

        if launcher_path.is_dir() and not (launcher_path / "main.sh").exists():
            raise TierkreisError(f"Expected launcher file. Got {launcher_path}.")

        if launcher_path.is_dir() and not (launcher_path / "main.sh").is_file():
            raise TierkreisError(f"Expected launcher file. Got {launcher_path}/main.sh")

        if launcher_path.is_dir() and (launcher_path / "main.sh").is_file():
            launcher_path = launcher_path / "main.sh"

        with open(self.workflow_dir.parent / worker_call_args_path) as fh:
            call_args = WorkerCallArgs(**json.load(fh))

        env = os.environ.copy() | self.env.copy()
        env.update(self._create_env(call_args, self.workflow_dir.parent, export_values))
        env["worker_call_args_file"] = str(
            self.workflow_dir.parent / worker_call_args_path
        )
        done_path = self.workflow_dir.parent / call_args.done_path
        _error_path = self.errors_path.parent / "_error"
        if TKR_DIR_KEY not in env:
            env[TKR_DIR_KEY] = str(self.logs_path.parent.parent)
        tee_str = f">(tee -a {str(self.errors_path)} {str(self.logs_path)} >/dev/null)"
        proc = subprocess.Popen(
            ["bash"],
            start_new_session=True,
            stdin=subprocess.PIPE,
            env=env,
        )
        proc.communicate(
            f"({launcher_path} {worker_call_args_path} > {tee_str} 2> {tee_str} && touch {done_path}|| touch {_error_path})&".encode(),
            timeout=self.timeout,
        )

    def _create_env(
        self, call_args: WorkerCallArgs, base_dir: Path, export_values: bool
    ) -> dict[str, str]:
        env = {
            "checkpoints_directory": str(base_dir),
            "function_name": str(base_dir / call_args.function_name),
            "done_path": str(base_dir / call_args.done_path),
            "error_path": str(base_dir / call_args.error_path),
            "output_dir": str(base_dir / call_args.output_dir),
        }
        if call_args.logs_path is not None:
            env["logs_path"] = str(base_dir / call_args.logs_path)
        else:
            env["logs_path"] = str(self.logs_path)
        env |= {
            f"output_{k}_file": str(base_dir / v) for k, v in call_args.outputs.items()
        }
        env |= {
            f"input_{k}_file": str(base_dir / v) for k, v in call_args.inputs.items()
        }
        if not export_values:
            return env
        values = {}
        for k, v in call_args.inputs.items():
            with open(v) as fh:
                values[f"input_{k}_value"] = fh.read()
        return env
