"""Executor for HPC schedulers through psij."""

import logging
import os
import tempfile
from copy import deepcopy
from pathlib import Path
from typing import Literal

from psij import InvalidJobException, Job, JobExecutor, SubmitException
from psij import JobSpec as PSIJJobSpec

from tierkreis.consts import TKR_DIR_KEY
from tierkreis.controller.executor.commands import add_std_handlers
from tierkreis.controller.executor.hpc.job_spec import (
    JobSpec,
)
from tierkreis.controller.executor.hpc.psij_conversion import spec_to_psij
from tierkreis.controller.storage.data import ExecutorDebugData
from tierkreis.exceptions import TierkreisError

logger = logging.getLogger(__name__)


class PSIJExecutor:
    def __init__(
        self,
        launchers_path: Path | None,
        logs_path: Path,
        spec: JobSpec | PSIJJobSpec,
        psij_executor: Literal["pbs", "slurm"],
    ) -> None:
        self.logs_path = logs_path
        self.errors_path = logs_path
        if isinstance(spec, JobSpec):
            self.spec = spec_to_psij(
                spec,
            )
        else:
            self.spec = spec
        self.psij_executor = JobExecutor.get_instance(psij_executor)

    def run(
        self,
        launcher_name: str,
        worker_call_args_path: Path,
    ) -> ExecutorDebugData:
        self.errors_path = (
            self.logs_path.parent.parent / worker_call_args_path.parent / "logs"
        )
        logger.info("START %s %s", launcher_name, worker_call_args_path)
        local_spec = deepcopy(self.spec)
        command = local_spec.executable or "uv"
        if local_spec.arguments is None:
            local_spec.arguments = ["run", "main.py"]
        command = (
            command
            + " "
            + " ".join(local_spec.arguments)
            + " "
            + str(worker_call_args_path)
        )

        local_spec.stdout_path = self.logs_path
        local_spec.stderr_path = self.errors_path
        local_spec.arguments = [
            add_std_handlers(self.logs_path, self.errors_path, command)
        ]
        if local_spec.environment is None:  # User can override by setting TKR_DIR
            local_spec.environment = {TKR_DIR_KEY: str(self.logs_path.parent.parent)}
        elif TKR_DIR_KEY not in local_spec.environment:
            local_spec.environment[TKR_DIR_KEY] = str(self.logs_path.parent.parent)

        # Create temporary eval script in TKR_DIR
        tkr_dir = Path(local_spec.environment[TKR_DIR_KEY])
        with tempfile.NamedTemporaryFile(
            mode="w", dir=str(tkr_dir), prefix="tkr_eval_", suffix=".sh", delete=False
        ) as exec_file:
            exec_file.write('#!/bin/bash\neval "$@"\n')
            exec_file.close()
            os.chmod(exec_file.name, 0o755)

        local_spec.executable = exec_file.name
        job = Job(local_spec)
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
            executor=f"{self.__class__!s}:{self.psij_executor.__class__!s}",
            launch_command=local_spec.executable
            + " "
            + " ".join(local_spec.arguments or []),
            job_id=job.native_id,
        )
