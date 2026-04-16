from tests.executor.stubs import mpi_fail, mpi_rank_info
from tierkreis.builder import Graph
from tierkreis.controller.data.graph import GraphData
from tierkreis.controller.data.models import TKR
from tierkreis.controller.executor.hpc.job_spec import (
    JobSpec,
    MpiSpec,
    ResourceSpec,
)


def mpi_graph() -> GraphData:
    builder = Graph(outputs_type=TKR[str | None])
    mpi_result = builder.task(mpi_rank_info())
    builder.finish_with_outputs(mpi_result)
    return builder.data


def mpi_fail_graph() -> GraphData:
    builder = Graph(outputs_type=TKR[None])
    mpi_result = builder.task(mpi_fail())
    builder.finish_with_outputs(mpi_result)
    return builder.data


def slurm_spec() -> JobSpec:
    return JobSpec(
        job_name="test_job",
        account="test_usr",
        command=("/root/.local/bin/uv run /mpi_worker/main.py"),
        resource=ResourceSpec(nodes=2, memory_gb=None),
        walltime="00:15:00",
        mpi=MpiSpec(max_proc_per_node=1),
        extra_scheduler_args={"--open-mode=append": None},
        environment={
            "OMPI_ALLOW_RUN_AS_ROOT": "1",
            "OMPI_ALLOW_RUN_AS_ROOT_CONFIRM": "1",
        },
    )


def pbs_spec() -> JobSpec:
    return JobSpec(
        job_name="test_job",
        account="pbsuser",
        command=("/home/pbsuser/.local/bin/uv run /mpi_worker/main.py"),
        resource=ResourceSpec(nodes=2, memory_gb=None, gpus_per_node=None),
        walltime="00:15:00",
        mpi=MpiSpec(max_proc_per_node=1),
        extra_scheduler_args={"-l place=scatter": None},
    )
