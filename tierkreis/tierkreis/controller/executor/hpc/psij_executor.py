"""Executor for HPC schedulers through psij."""

import logging
import os
import tempfile
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

        command = self.spec.executable or "uv"
        if self.spec.arguments is None:
            self.spec.arguments = ["run", "main.py"]
        command = (
            command
            + " "
            + " ".join(self.spec.arguments)
            + " "
            + str(worker_call_args_path)
        )

        self.spec.stdout_path = self.logs_path
        self.spec.stderr_path = self.errors_path
        self.spec.arguments = [
            add_std_handlers(self.logs_path, self.errors_path, command)
        ]
        if self.spec.environment is None:  # User can override by setting TKR_DIR
            self.spec.environment = {TKR_DIR_KEY: str(self.logs_path.parent.parent)}
        elif TKR_DIR_KEY not in self.spec.environment:
            self.spec.environment[TKR_DIR_KEY] = str(self.logs_path.parent.parent)

        # Create temporary eval script in TKR_DIR
        tkr_dir = Path(self.spec.environment[TKR_DIR_KEY])
        with tempfile.NamedTemporaryFile(
            mode="w", dir=str(tkr_dir), prefix="tkr_eval_", suffix=".sh", delete=False
        ) as exec_file:
            exec_file.write('#!/bin/bash\neval "$@"\n')
            exec_file.close()
            os.chmod(exec_file.name, 0o755)

        job = Job(self.spec)
        self.spec.executable = exec_file.name

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
            launch_command=self.spec.executable
            + " "
            + " ".join(self.spec.arguments or []),
            job_id=job.native_id,
        )
