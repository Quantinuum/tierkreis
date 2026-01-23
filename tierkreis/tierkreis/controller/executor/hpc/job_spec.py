from pathlib import Path
import platform
from pydantic import BaseModel, Field


class MpiSpec(BaseModel):
    max_proc_per_node: int | None = 1
    proc: int | None = None


class ResourceSpec(BaseModel):
    nodes: int = 1
    cores_per_node: int | None = 1
    memory_gb: int | None = 4
    gpus_per_node: int | None = 0


class UserSpec(BaseModel):
    mail: str | None = None  # some clusters require this


class ContainerSpec(BaseModel):
    image: str
    engine: str  # e.g. singularity, docker, enroot?
    name: str | None = None
    extra_args: dict[str, str | None] = Field(default_factory=dict)
    env_file: str | None = None


class JobSpec(BaseModel):
    job_name: str
    command: str  # used instead of popen.input
    resource: ResourceSpec
    account: str | None = None
    """Account or group used to submit this job."""
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
    arch = platform.machine()
    uv_path = Path.home() / ".local" / f"bin_{arch}" / "uv"
    return JobSpec(
        job_name="pjsub_large",
        account="hp240496",
        command=f"{str(uv_path)} run main.py",
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
    arch = platform.machine()
    uv_path = Path.home() / ".local" / f"bin_{arch}" / "uv"
    return JobSpec(
        job_name="pjsub_small",
        account="hp240496",
        command=f"{str(uv_path)} run main.py",
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
