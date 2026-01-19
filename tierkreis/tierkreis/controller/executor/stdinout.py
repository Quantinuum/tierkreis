"""Special case implementation for external workers."""

# ruff: noqa: D102 (class methods inherited from ControllerExecutor)
import json
import shutil
import subprocess
from pathlib import Path

from tierkreis.controller.data.location import WorkerCallArgs
from tierkreis.controller.executor.check_launcher import check_and_set_launcher


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

    def __init__(self, registry_path: Path, workflow_dir: Path) -> None:
        self.launchers_path = registry_path
        self.logs_path = workflow_dir / "logs"
        self.errors_path = workflow_dir / "logs"
        self.workflow_dir = workflow_dir

    def run(
        self,
        launcher_name: str,
        worker_call_args_path: Path,
    ) -> None:
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

        tee_str = f">(tee -a {self.errors_path!s} {self.logs_path!s} >/dev/null)"
        _error_path = done_path.parent / "_error"
        proc = subprocess.Popen(
            ["/bin/bash"],
            start_new_session=True,
            stdin=subprocess.PIPE,
        )
        proc.communicate(
            f"({launcher_path} <{input_file}  > {output_file} 2> {tee_str}"
            f" && touch {done_path}|| touch {_error_path})&".encode(),
            timeout=10,
        )


def _check_bin(command: str) -> str | None:
    path = shutil.which(command)
    if path is None:
        return None
    return command
