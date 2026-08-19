"""Special case implementation for external workers."""

import json
import os
import shutil
import subprocess
from pathlib import Path

from tierkreis.controller.data.location import WorkerCallArgs
from tierkreis.controller.executor.check_launcher import check_and_set_launcher
from tierkreis.controller.storage.data import ExecutorDebugData


class StdInOut:
    """Executes workers in an unix shell.

    Assumes the worker takes a single input from stdin and will produce a single output
    to stdout.
    Will pipe other outputs to errors / logs.
    Works by creating a subprocess
    Implements: :py:class:`tierkreis.controller.executor.protocol.ControllerExecutor`

    :fields:
        launchers_path (Path): The locations to search for external workers.
        logs_path (Path): The controller log file.
        errors_path (Path): The controller error file for the function node.
        workflow_dir (Path): The workflow dir to resolve relative paths.
    """

    def __init__(
        self,
        registry_path: Path,
        workflow_dir: Path,
        log_env_in_debug: bool = True,
    ) -> None:
        self.launchers_path = registry_path
        self.logs_path = workflow_dir / "logs"
        self.errors_path = workflow_dir / "logs"
        self.workflow_dir = workflow_dir
        self.log_env_in_debug = log_env_in_debug

    def run(
        self,
        launcher_name: str,
        worker_call_args_path: Path,
    ) -> ExecutorDebugData:
        launcher_path = _check_bin(launcher_name)
        if launcher_path is None:
            launcher_path = check_and_set_launcher(
                self.launchers_path,
                launcher_name,
                ".sh",
            )

        with Path.open(self.workflow_dir.parent / worker_call_args_path) as fh:
            call_args = WorkerCallArgs(**json.load(fh))

        input_file = self.workflow_dir.parent / next(iter(call_args.inputs.values()))
        output_file = self.workflow_dir.parent / next(iter(call_args.outputs.values()))
        done_path = self.workflow_dir.parent / call_args.done_path
        self.errors_path = done_path.parent / "logs"
        _error_path = self.errors_path.parent / "_error"

        tee_str = f">(tee -a {self.errors_path!s} {self.logs_path!s} >/dev/null)"
        _error_path = done_path.parent / "_error"
        proc = subprocess.Popen(
            ["bash"],
            start_new_session=True,
            stdin=subprocess.PIPE,
        )
        command = f"({launcher_path} <{input_file}  > {output_file} 2> {tee_str} && touch {done_path}|| touch {_error_path})&"
        proc.communicate(
            f"({launcher_path} <{input_file}  > {output_file} 2> {tee_str}"
            f" && touch {done_path}|| touch {_error_path})&".encode(),
            timeout=10,
        )
        return self._generate_debug_data(command)

    def _generate_debug_data(self, command: str) -> ExecutorDebugData:
        if not self.log_env_in_debug:
            env = {}
        else:
            env = {k: v for k, v in os.environ.items()}
        # What is the equivalent to the pip freeze here?
        return ExecutorDebugData(
            executor=str(self.__class__),
            launch_command=command,
            env=env,
        )


def _check_bin(command: str) -> str | None:
    path = shutil.which(command)
    if path is None:
        return None
    return command
