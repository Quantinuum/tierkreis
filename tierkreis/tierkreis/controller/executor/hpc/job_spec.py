"""Definition of HPC resource classes.

These are used to map resource requirements to the respective
settings in resource management systems (schedulers).
Using the value `None` typically will unset the flag.
It is not guaranteed that all schedulers can realize all the configurations.
"""

import platform
from pathlib import Path

from pydantic import BaseModel, Field


class MpiSpec(BaseModel):
    """MPI configuration.

    :fields:
        max_proc_per_node (int | None): Number of MPI processes per compute node,
            defaults to 1.
        proc (int | None): Number of MPI processes (ranks), defaults to None (unset).

    """

    max_proc_per_node: int | None = 1
    proc: int | None = None


class ResourceSpec(BaseModel):
    """General resource definitions.

    :fields:
        nodes (int): Number of compute nodes, defaults to 1.
        cores_per_node (int | None): Number of cores to use per node, defaults to 1.
        memory_gb (int | None): Memory per node in GB, defaults to 4.
        gpus_per_nod (int | None): Physical GPUs to reserve on the node, defaults to 0.
    """

    nodes: int = 1
    cores_per_node: int | None = 1
    memory_gb: int | None = 4
    gpus_per_node: int | None = 0


class UserSpec(BaseModel):
    """User specific configuration.

    :fields:
        mail (str | None): User email to send job updates.
    """

    mail: str | None = None  # some clusters require this


class ContainerSpec(BaseModel):
    """Configuration for the use of container images in HPC.

    :warning:
    Not fully supported yet.

    :fields:
        images (str): URL to the container image.
        engine (str): which engine to use.
        name (str | None): Explicit image name, defaults to None.
        extra_args (dict[str, str | None]): Environment args to pass to the container,
            defaults to {}.
        env_file (str | None): Path to a file with variable export definitions,
            defaults to None.
    """

    image: str
    engine: str  # e.g. singularity, docker, enroot?
    name: str | None = None
    extra_args: dict[str, str | None] = Field(default_factory=dict)
    env_file: str | None = None


class JobSpec(BaseModel):
    """Resource definition for an HPC job.

    This is used to generate the job script for the scheduler.

    :fields:
        job_name (str): Reference name for the job.
        command (str): The command to execute on hpc.
            E.g. "mpi run ..."
        resource (ResourceSpec): Resource specification for the job.
        account: (str | None): Account or group used to submit this job,
            defaults to None.
        mpi: (MpiSpec | None): The MPI specification. If this is set, will prepend
            "mpirun" to the command string, defaults to None.
        container: (ContainerSpec | None): The container specification for the job,
            defaults to None.
        walltime (str): Maximum walltime of the job in HH:MM:SS format,
            defaults to "01:00:00".
        queue: (str | None): Named queue to submit to, HPC center specific,
            defaults to None.
        output_path: (Path | None): Explicit job output, if not tkr output will be used,
            defaults to None.
        error_path: (Path | None): Explicit error output, if not tkr output will be used
            defaults to None.
        extra_scheduler_args (dict[str, str | None]): Configure additional flags and
            options that are not provided in the spec. Flags are set as
            extra_scheduler_args["flag_name"] = None, options set as
            extra_scheduler_args["option_name"] = "option_value".
            Defaults to {}
        environment: (dict[str, str]): Provide additional environment variables to the
            job, defaults to {}.
        include_no_check_directory_flag: (bool): Set "--no-check-directory",
            defaults to false.
    """

    job_name: str
    command: str  # used instead of popen.input
    resource: ResourceSpec
    account: str | None = None
    mpi: MpiSpec | None = None
    user: UserSpec | None = None
    container: ContainerSpec | None = None
    walltime: str = "01:00:00"  # HH:MM:SS Replacement?
    queue: str | None = None
    output_path: Path | None = None
    error_path: Path | None = None
    extra_scheduler_args: dict[str, str | None] = Field(default_factory=dict)
    environment: dict[str, str] = Field(default_factory=dict)
    include_no_check_directory_flag: bool = False


def pjsub_large_spec() -> JobSpec:
    """Generate an example large job specification for FUGAKU.

    :return: A job spec running uv on FUGAKU.
    :rtype: JobSpec
    """
    arch = platform.machine()
    uv_path = Path.home() / ".local" / f"bin_{arch}" / "uv"
    return JobSpec(
        job_name="pjsub_large",
        account="hp240496",
        command=f"{uv_path!s} run main.py",
        queue="q-QTM-M",
        resource=ResourceSpec(nodes=32),
        environment={
            "VIRTUAL_ENVIRONMENT": "",
            "UV_PROJECT_ENVIRONMENT": f"venv_{arch}",
            "PJM_LLIO_GFSCACHE": "/vol0004",
        },
        walltime="03:00:00",
        extra_scheduler_args={
            "-z": "jid",
            "--llio": "sharedtmp-size=64Gi",
        },
        mpi=MpiSpec(max_proc_per_node=4),
    )


def pjsub_small_spec() -> JobSpec:
    """Generate an example small job specification for FUGAKU.

    :return: A job spec running uv on FUGAKU.
    :rtype: JobSpec
    """
    arch = platform.machine()
    uv_path = Path.home() / ".local" / f"bin_{arch}" / "uv"
    return JobSpec(
        job_name="pjsub_small",
        account="hp240496",
        command=f"{uv_path!s} run main.py",
        queue="q-QTM-S",
        resource=ResourceSpec(nodes=1),
        environment={
            "VIRTUAL_ENVIRONMENT": "",
            "UV_PROJECT_ENVIRONMENT": f"venv_{arch}",
        },
        walltime="00:15:00",
        extra_scheduler_args={
            "-z": "jid",
            "--llio": "sharedtmp-size=10Gi",
        },
        mpi=MpiSpec(max_proc_per_node=4),
    )
