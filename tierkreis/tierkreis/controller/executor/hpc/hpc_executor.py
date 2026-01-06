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
    registry_path: Path | None
    logs_path: Path
    errors_path: Path | None
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
    logging.basicConfig(
        format="%(asctime)s: %(message)s",
        datefmt="%Y-%m-%dT%H:%M:%S%z",
        filename=executor.logs_path,
        filemode="a",
        level=logging.INFO,
        force=True,
    )
    logger.info("START %s %s", launcher_name, worker_call_args_path)

    spec = executor.spec.model_copy()
    if executor.registry_path:
        spec.command = f"cd {executor.registry_path}/{launcher_name} && {spec.command}"

    spec.command += " " + str(worker_call_args_path)
    submission_cmd = [executor.command]
    if spec.output_path is None:
        submission_cmd += ["-o", str(executor.logs_path)]
    else:
        submission_cmd += ["-o", str(spec.output_path)]
    if spec.error_path is None:
        submission_cmd += ["-e", str(executor.errors_path)]
    else:
        submission_cmd += ["-e", str(spec.error_path)]
    if spec.include_no_check_directory_flag:
        submission_cmd += ["--no-check-directory"]

    if TKR_DIR_KEY not in spec.environment:  # User can override by setting TKR_DIR
        spec.environment[TKR_DIR_KEY] = str(executor.logs_path.parent.parent)

    with NamedTemporaryFile(
        mode="w+",
        delete=True,
        suffix=".sh",
        prefix=f"{spec.job_name}-",
    ) as script_file:
        generate_script(executor.script_fn, spec, Path(script_file.name))
        submission_cmd.append(script_file.name)

        process = subprocess.run(
            submission_cmd,
            start_new_session=True,
            capture_output=True,
            universal_newlines=True,
        )
    if process.returncode != 0:
        with open(executor.errors_path, "a") as efh:
            efh.write("Error from script")
            efh.write(process.stderr)
        raise TierkreisError(f"Executor failed with return code {process.returncode}")
    logger.info("Submitted job with return code %s", process.stdout.rstrip())
