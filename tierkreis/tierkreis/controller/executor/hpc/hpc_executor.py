"""Interface implementation for HPCExecutors."""

import logging
import subprocess
from collections.abc import Callable
from pathlib import Path
from tempfile import NamedTemporaryFile
from typing import Protocol

from tierkreis.consts import TKR_DIR_KEY
from tierkreis.controller.executor.commands import add_std_handlers
from tierkreis.controller.executor.hpc.job_spec import JobSpec
from tierkreis.controller.storage.data import ExecutorDebugData
from tierkreis.exceptions import TierkreisError

logger = logging.getLogger(__name__)


class HPCExecutor(Protocol):
    """Generic protocol for an HPC executor.

    :fields:
        launchers_path (Path | None): The locations to search for workers.
            This will change the location from where the command is invoked
            by appending "cd launchers_path && "
        logs_path (Path): The controller log file.
        errors_path (Path): The controller error file for the function node.
        spec (JobSpec): A definition of the job specification.
        script_fn (Callable[[JobSpec], str]): A template function to generate the
            submission script from.
        command (str): The base command to use.
    """

    launchers_path: Path | None
    logs_path: Path
    errors_path: Path
    spec: JobSpec
    script_fn: Callable[[JobSpec], str]
    command: str

    def job_id(self, std_out: str) -> str: ...


def generate_script(
    template_fn: Callable[[JobSpec], str],
    spec: JobSpec,
    path: Path,
) -> None:
    """Generate a scheduler script by calling a template function.

    :param template_fn: The template function to call.
    :type template_fn: Callable[[JobSpec], str]
    :param spec: The job definition to generate the script for.
    :type spec: JobSpec
    :param path: The path to save the script to.
    :type path: Path
    """
    with Path.open(path, "w+", encoding="utf-8") as fh:
        fh.write(template_fn(spec))


def run_hpc_executor(
    executor: HPCExecutor,
    launcher_name: str,
    worker_call_args_path: Path,
    *,
    run_tkr=True,
) -> ExecutorDebugData:
    """Run a worker function on an HPC executor.

    This is a generic function to run with with an HPC executor.
    Similar to the :py:class:`tierkreis.controller.executor.protocol.ControllerExecutor`
    run function.

    :param executor: The executor to use for running
    :type executor: HPCExecutor
    :param launcher_name: Module description for the worker to run
    :type launcher_name: str
    :param worker_call_args_path: Location of the worker call args.
    :type worker_call_args_path: Path
    :raises TierkreisError: When job submission fails.
    """
    logger.info("START %s %s", launcher_name, worker_call_args_path)

    spec = executor.spec.model_copy()

    if run_tkr:
        spec.command = f"cd {executor.launchers_path} && {spec.command} tkr-{launcher_name.replace('_', '-')}"

    elif executor.launchers_path:
        spec.command = f"cd {executor.launchers_path}/{launcher_name} && {spec.command}"

    spec.command += " " + str(worker_call_args_path)
    spec.command = add_std_handlers(
        executor.logs_path,
        executor.errors_path,
        spec.command,
        with_parentheses=False,
    )
    submission_cmd = [executor.command]
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
            text=True,
            check=False,
        )

    with Path.open(executor.logs_path, "a+") as fh:
        fh.write(process.stdout)

    with Path.open(executor.errors_path, "a+") as fh:
        fh.write(process.stdout)

    if process.returncode != 0:
        with Path.open(executor.errors_path, "a") as efh:
            efh.write("Error from script")
            efh.write(process.stderr)
        print(process.stderr)
        print("\n\nsubmission script\n\n")
        print(executor.script_fn(spec))

        msg = f"Executor failed with return code {process.returncode}"
        raise TierkreisError(msg)

    return ExecutorDebugData(
        executor=str(executor.__class__),
        launch_command=" ".join(submission_cmd),
        job_id=executor.job_id(process.stdout),  # TODO parse JOB ID somehow
    )
