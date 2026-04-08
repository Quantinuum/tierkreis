"""Executor for HPC schedulers through psij."""

import logging
from pathlib import Path

from psij import InvalidJobException, Job, JobExecutor, JobSpec, SubmitException

from tierkreis.consts import TKR_DIR_KEY
from tierkreis.controller.executor.commands import add_std_handlers
from tierkreis.controller.storage.data import ExecutorDebugData
from tierkreis.exceptions import TierkreisError

logger = logging.getLogger(__name__)


class PSIJExecutor:
    def __init__(
        self,
        launchers_path: Path | None,
        logs_path: Path,
        spec: JobSpec,
        psij_executor: JobExecutor,
    ) -> None:
        self.launchers_path = launchers_path
        self.logs_path = logs_path
        self.errors_path = logs_path
        self.spec = spec
        self.psij_executor = psij_executor

    def run(
        self,
        launcher_name: str,
        worker_call_args_path: Path,
    ) -> ExecutorDebugData:
        self.errors_path = (
            self.logs_path.parent.parent / worker_call_args_path.parent / "logs"
        )
        logger.info("START %s %s", launcher_name, worker_call_args_path)
        command = self.spec.executable or "uv run main.py"
        if self.launchers_path:
            self.spec.directory = self.launchers_path / launcher_name

        command += " " + str(worker_call_args_path)
        command = add_std_handlers(
            self.logs_path,
            self.errors_path,
            command,
        )[1:-1]

        if self.spec.environment is None:  # User can override by setting TKR_DIR
            self.spec.environment = {TKR_DIR_KEY: str(self.logs_path.parent.parent)}
        elif TKR_DIR_KEY not in self.spec.environment:
            self.spec.environment[TKR_DIR_KEY] = str(self.logs_path.parent.parent)
        self.spec.executable = command
        job = Job(self.spec)

        try:
            self.psij_executor.submit(job)
            with Path.open(self.logs_path, "a+") as fh:
                fh.write(str(job.status))

            with Path.open(self.errors_path, "a+") as fh:
                fh.write(str(job.status))
        except (InvalidJobException, SubmitException) as e:
            with Path.open(self.errors_path, "a") as efh:
                efh.write("Error from script")
                efh.write(e.message)

            msg = "PSIJ Executor failed to submit job."
            raise TierkreisError(msg) from e

        return ExecutorDebugData(
            executor=f"{str(self.__class__)}:{str(self.psij_executor.__class__)}",
            launch_command=command,
            job_id=job.native_id,
        )
