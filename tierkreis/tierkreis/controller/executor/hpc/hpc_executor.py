import logging
import subprocess
from pathlib import Path
from tempfile import NamedTemporaryFile
from typing import Callable, Protocol

from tierkreis.consts import TKR_DIR_KEY
from tierkreis.controller.executor.hpc.job_spec import JobSpec
from tierkreis.exceptions import TierkreisError


logger = logging.getLogger(__name__)


class HPCExecutor(Protocol):
    launchers_path: Path | None
    logs_path: Path
    errors_path: Path
    spec: JobSpec
    script_fn: Callable[[JobSpec], str]
    command: str


def generate_script(
    template_fn: Callable[[JobSpec], str], spec: JobSpec, path: Path
) -> None:
    with open(path, "w+", encoding="utf-8") as fh:
        fh.write(template_fn(spec))


def run_hpc_executor(
    executor: HPCExecutor, launcher_name: str, worker_call_args_path: Path
) -> None:
    logger.info("START %s %s", launcher_name, worker_call_args_path)

    spec = executor.spec.model_copy()
    if executor.launchers_path:
        spec.command = f"cd {executor.launchers_path}/{launcher_name} && {spec.command}"

    spec.command += " " + str(worker_call_args_path)
    submission_cmd = [executor.command]
    submission_cmd.append("-j")  # Pipe stderr to the same place as stdout

    submission_cmd += ["-o", str(executor.errors_path)]
    if spec.include_no_check_directory_flag:
        submission_cmd += ["--no-check-directory"]

    if TKR_DIR_KEY not in spec.environment:  # User can override by setting TKR_DIR
        spec.environment[TKR_DIR_KEY] = str(executor.logs_path.parent.parent)

    executor.errors_path.touch(exist_ok=True)

    with NamedTemporaryFile(
        mode="w+",
        delete=True,
        suffix=".sh",
        prefix=f"{spec.job_name}-",
    ) as script_file:
        generate_script(executor.script_fn, spec, Path(script_file.name))
        submission_cmd.append(script_file.name)
        tee_str = f">(tee -a {str(executor.errors_path)} {str(executor.logs_path)} >/dev/null)"
        _error_path = executor.errors_path.parent / "_error"

        proc = subprocess.Popen(["bash"], start_new_session=True, stdin=subprocess.PIPE)
        subproc_cmd = f"({" ".join(submission_cmd)} > {tee_str} 2> {tee_str} || touch {_error_path}) &".encode()
        print(subproc_cmd)
        proc.communicate(subproc_cmd, timeout=10)

    with open(executor.errors_path) as fh:
        log_output = fh.read()

    with open(executor.logs_path, "a+") as fh:
        fh.write(log_output)

    if proc.returncode != 0:
        raise TierkreisError(f"Executor failed with return code {proc.returncode}")
    logger.info("Submitted job with return code %s", proc.stdout)
