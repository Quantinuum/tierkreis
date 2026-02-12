import json
import os
import shutil
import subprocess
from pathlib import Path

from tierkreis.controller.data.location import WorkerCallArgs
from tierkreis.controller.storage.data import ExecutorData
from tierkreis.controller.executor.check_launcher import check_and_set_launcher


class StdInOut:
    """Executes workers in an unix shell.

    Implements: :py:class:`tierkreis.controller.executor.protocol.ControllerExecutor`
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
    ) -> ExecutorData:
        launcher_path = _check_bin(launcher_name)
        if launcher_path is None:
            launcher_path = check_and_set_launcher(
                self.launchers_path, launcher_name, ".sh"
            )

        with open(self.workflow_dir.parent / worker_call_args_path) as fh:
            call_args = WorkerCallArgs(**json.load(fh))

        input_file = self.workflow_dir.parent / list(call_args.inputs.values())[0]
        output_file = self.workflow_dir.parent / list(call_args.outputs.values())[0]
        done_path = self.workflow_dir.parent / call_args.done_path

        tee_str = f">(tee -a {str(self.errors_path)} {str(self.logs_path)} >/dev/null)"
        _error_path = done_path.parent / "_error"
        proc = subprocess.Popen(
            ["bash"],
            start_new_session=True,
            stdin=subprocess.PIPE,
        )
        command = f"({launcher_path} <{input_file}  > {output_file} 2> {tee_str} && touch {done_path}|| touch {_error_path})&"
        proc.communicate(
            command.encode(),
            timeout=10,
        )
        return self._generate_debug_data(command)

    def _generate_debug_data(self, command: str) -> ExecutorData:
        # What is the equivalent to the pip freeze here?
        return ExecutorData(
            str(self.__class__),
            command,
            env={k: v for k, v in os.environ.items()},
        )


def _check_bin(command: str) -> str | None:
    path = shutil.which(command)
    if path is None:
        return None
    return command
