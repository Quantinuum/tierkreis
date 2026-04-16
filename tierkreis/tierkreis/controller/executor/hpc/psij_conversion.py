import logging
import shlex
from datetime import timedelta
from typing import Any

import psij

from tierkreis.controller.executor.hpc.job_spec import (
    JobSpec,
    MpiSpec,
    ResourceSpec,
    UserSpec,
)
from tierkreis.exceptions import TierkreisError

logger = logging.getLogger(__name__)


def _parse_walltime(walltime_str: str) -> timedelta:
    parts = list(map(int, walltime_str.split(":")))
    if len(parts) >= 3:
        return timedelta(hours=parts[0], minutes=parts[1], seconds=parts[2])
    return timedelta()


def _format_walltime(delta: timedelta) -> str:
    total_seconds = int(delta.total_seconds())
    hours = total_seconds // 3600
    minutes = (total_seconds % 3600) // 60
    seconds = total_seconds % 60
    return f"{hours:02d}:{minutes:02d}:{seconds:02d}"


def spec_to_psij(spec: JobSpec, target_scheduler: str | None = None) -> psij.JobSpec:
    """Convert a Tierkreis JobSpec into a psij.JobSpec.

    Conversion tries to map all fields as closely as possible.
    Especially extra_scheduler_args can be tricky and might not be correctly mapped.

    :param spec: The tierkreis.JobSpec to convert.
    :type spec: tierkreis.JobSpec
    :param target_scheduler: The target scheduler for the job, defaults to None.
    :type target_scheduler: str | None
    :return: The converted psij.JobSpec.
    :rtype: psij.JobSpec
    """
    cmd_parts = shlex.split(spec.command)

    res_spec = psij.ResourceSpecV1(
        node_count=spec.resource.nodes,
        process_count=spec.mpi.proc if spec.mpi else None,
        processes_per_node=spec.mpi.max_proc_per_node if spec.mpi else None,
        cpu_cores_per_process=spec.resource.cores_per_node,
        gpu_cores_per_process=spec.resource.gpus_per_node,
        memory=spec.resource.memory_gb * 1024 * 1024 * 1024
        if spec.resource.memory_gb
        else None,
    )

    # 2. Map Attributes and capture custom fields that psij doesn't natively support
    custom_attrs: dict[str, Any] = {}
    if spec.extra_scheduler_args:
        if target_scheduler is None:
            logger.warning(
                "extra_scheduler_args provided without target_scheduler."
                " These will be included as custom attributes but may not be correctly interpreted by psij."
            )
            custom_attrs.update(
                {k.strip("-"): v for k, v in spec.extra_scheduler_args.items()}
            )
        else:
            custom_attrs.update(
                {
                    target_scheduler + "." + k.strip("-"): v
                    for k, v in spec.extra_scheduler_args.items()
                }
            )

    if spec.user and spec.user.mail:
        logger.warning("User email is not natively supported in psij.")
        if target_scheduler == "slurm":
            custom_attrs["slurm.mail-user"] = spec.user.mail
        elif target_scheduler == "pbs":
            custom_attrs["pbs.m"] = spec.user.mail
        else:
            logger.warning(
                "No specific mapping for user email in target scheduler available. Try storing as a custom attribute."
            )
    if spec.container:
        msg = (
            "ContainerSpec is not supported by psij. Please use an alternative executor"
        )
        raise TierkreisError(msg)

    attributes = psij.JobAttributes(
        duration=_parse_walltime(spec.walltime),
        queue_name=spec.queue,
        account=spec.account,
        custom_attributes=custom_attrs if custom_attrs else None,
    )
    environment: dict[str, str | int] = {}
    for k, v in spec.environment.items():
        try:
            environment[k] = int(v)
        except (ValueError, TypeError):
            environment[k] = v

    return psij.JobSpec(
        name=spec.job_name,
        executable=cmd_parts[0] if cmd_parts else None,
        arguments=cmd_parts[1:] if cmd_parts else None,
        environment=environment,
        stdout_path=spec.output_path,
        stderr_path=spec.error_path,
        resources=res_spec,
        attributes=attributes,
        launcher="mpirun" if spec.mpi else "single",
    )


def psij_to_spec(psij_spec: psij.JobSpec) -> JobSpec:
    """Convert a psij.JobSpec into a tierkreis.JobSpec.

    :param psij_spec: The psij.JobSpec to convert.
    :type psij_spec: psij.JobSpec
    :return: The converted tierkreis.JobSpec.
    :rtype: JobSpec
    """

    command = psij_spec.executable or ""
    if psij_spec.arguments:
        command = shlex.join([command] + psij_spec.arguments).strip()

    if psij_spec.resources is not None and isinstance(
        psij_spec.resources, psij.ResourceSpecV1
    ):
        memory_gb = (
            (psij_spec.resources.memory // (1024 * 1024 * 1024))
            if psij_spec.resources.memory
            else None
        )
        resource = ResourceSpec(
            nodes=psij_spec.resources.node_count or 1,
            cores_per_node=psij_spec.resources.cpu_cores_per_process,
            gpus_per_node=psij_spec.resources.gpu_cores_per_process,
            memory_gb=memory_gb,
        )
    else:
        resource = ResourceSpec()

    # 3. Map MPI
    mpi = None
    if psij_spec.launcher in ["mpirun", "srun", "aprun"]:
        max_ppn = 1
        proc = None
        if isinstance(psij_spec.resources, psij.ResourceSpecV1):
            max_ppn = psij_spec.resources.processes_per_node
            proc = psij_spec.resources.process_count
        mpi = MpiSpec(max_proc_per_node=max_ppn, proc=proc)

    account, queue, walltime = None, None, "01:00:00"
    extra_args, user = {}, None
    if psij_spec.attributes:
        account = psij_spec.attributes.account
        queue = psij_spec.attributes.queue_name
        if psij_spec.attributes.duration:
            walltime = _format_walltime(psij_spec.attributes.duration)

        extra_args: dict[str, str | None] = {}
        if psij_spec.attributes.custom_attributes:
            for k, v in psij_spec.attributes.custom_attributes.items():
                if k in ["slurm.mail-user", "pbs.m"]:
                    user = UserSpec(mail=str(v))
                    continue
                if len(new_key := k.split(".", 1)) == 1:
                    logging.warning(
                        f"Custom attribute '{k}' does not follow expected 'scheduler.key' format. Storing as-is in extra_scheduler_args."
                    )
                    extra_args[f"--{k}"] = str(v) if v is not None else None
                else:
                    extra_args[f"--{new_key[1]}"] = str(v) if v is not None else None

    # 5. Construct custom.JobSpec
    return JobSpec(
        job_name=psij_spec.name or "psij_job",
        command=command,
        resource=resource,
        account=account,
        mpi=mpi,
        user=user,
        walltime=walltime,
        queue=queue,
        output_path=psij_spec.stdout_path,
        error_path=psij_spec.stderr_path,
        extra_scheduler_args=extra_args,
        environment=psij_spec.environment or {},
    )
