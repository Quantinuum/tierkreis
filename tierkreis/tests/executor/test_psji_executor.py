from uuid import UUID

import pytest

from tests.executor.stubs import mpi_fail, mpi_rank_info
from tierkreis.builder import Graph
from tierkreis.controller import run_graph
from tierkreis.controller.data.graph import GraphData
from tierkreis.controller.data.models import TKR
from tierkreis.controller.executor.hpc.job_spec import (
    JobSpec,
    MpiSpec,
    ResourceSpec,
)
from tierkreis.controller.executor.hpc.psij_executor import PSIJExecutor
from tierkreis.controller.storage.filestorage import ControllerFileStorage
from tierkreis.exceptions import TierkreisError
from tierkreis.storage import read_outputs


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
        command=("/root/.local/bin/uv run /mpi_worker/main.py "),
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
        command=("/home/pbsuser/.local/bin/uv run /mpi_worker/main.py "),
        resource=ResourceSpec(nodes=2, memory_gb=None, gpus_per_node=None),
        walltime="00:15:00",
        mpi=MpiSpec(max_proc_per_node=1),
        extra_scheduler_args={"-l place=scatter": None},
    )


@pytest.mark.skip(reason="Needs SLURM setup.")
def test_psij_slurm_mpi() -> None:
    g = mpi_graph()
    storage = ControllerFileStorage(
        UUID(int=24),
        name="psij_mpi_graph",
        do_cleanup=True,
    )
    executor = PSIJExecutor(
        spec=slurm_spec(),
        launchers_path=None,
        logs_path=storage.logs_path,
        psij_executor="slurm",
    )
    run_graph(storage, executor, g, {})

    output = read_outputs(g, storage)

    assert output is not None
    assert output == "Rank 0 out of 2 on c1.\nRank 1 out of 2 on c2."


@pytest.mark.skip(reason="Needs SLURM setup.")
def test_psij_slurm_mpi_fail() -> None:
    g = mpi_fail_graph()
    storage = ControllerFileStorage(
        UUID(int=25),
        name="psij_mpi_failing_graph",
        do_cleanup=True,
    )
    executor = PSIJExecutor(
        spec=slurm_spec(),
        launchers_path=None,
        logs_path=storage.logs_path,
        psij_executor="slurm",
    )
    with pytest.raises(TierkreisError):
        run_graph(storage, executor, g, {})

    assert storage.exists(storage.workflow_dir / "-.N0/_error")


@pytest.mark.skip(reason="Needs PBS setup.")
def test_psij_pbs_mpi() -> None:
    g = mpi_graph()
    storage = ControllerFileStorage(
        UUID(int=24),
        name="psij_mpi_graph",
        do_cleanup=True,
    )
    executor = PSIJExecutor(
        spec=pbs_spec(),
        launchers_path=None,
        logs_path=storage.logs_path,
        psij_executor="pbs",
    )
    run_graph(storage, executor, g, {})

    output = read_outputs(g, storage)

    assert output is not None
    assert output == "Rank 0 out of 2 on p1.\nRank 1 out of 2 on p2."


@pytest.mark.skip(reason="Needs PBS setup.")
def test_psij_pbs_mpi_fail() -> None:
    g = mpi_fail_graph()
    storage = ControllerFileStorage(
        UUID(int=25),
        name="psij_mpi_failing_graph",
        do_cleanup=True,
    )
    executor = PSIJExecutor(
        spec=pbs_spec(),
        launchers_path=None,
        logs_path=storage.logs_path,
        psij_executor="pbs",
    )
    with pytest.raises(TierkreisError):
        run_graph(storage, executor, g, {})

    assert storage.exists(storage.workflow_dir / "-.N0/_error")
