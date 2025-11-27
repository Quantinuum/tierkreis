import json
import os
import subprocess
from pathlib import Path

from tierkreis.consts import TKR_DIR_KEY
from tierkreis.controller.data.location import WorkerCallArgs
from tierkreis.controller.executor.check_launcher import check_and_set_launcher


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
        self.errors_path = worker_call_args_path.parent / "errors"

        launcher_path = check_and_set_launcher(
            self.launchers_path, launcher_name, ".sh"
        )

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

        with open(self.workflow_dir.parent / self.logs_path, "a") as lfh:
            with open(self.workflow_dir.parent / self.errors_path, "a") as efh:
                proc = subprocess.Popen(
                    ["bash"],
                    start_new_session=True,
                    stdin=subprocess.PIPE,
                    stderr=efh,
                    stdout=lfh,
                    env=env,
                )
                proc.communicate(
                    f"({launcher_path} {worker_call_args_path} && touch {done_path}|| touch {_error_path})&".encode(),
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
