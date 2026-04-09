from datetime import timedelta
from pathlib import Path
from uuid import UUID


from tests.executor.stubs import mpi_rank_info
from tierkreis.builder import Graph
from tierkreis.controller import run_graph
from tierkreis.controller.data.graph import GraphData
from tierkreis.controller.data.models import TKR
from tierkreis.controller.executor.hpc.psij_executor import PSIJExecutor
from tierkreis.controller.storage.filestorage import ControllerFileStorage
from tierkreis.storage import read_outputs

from psij import JobAttributes, JobExecutor, JobSpec, ResourceSpecV1


def mpi_graph() -> GraphData:
    builder = Graph(outputs_type=TKR[str | None])
    mpi_result = builder.task(mpi_rank_info())
    builder.finish_with_outputs(mpi_result)
    return builder.data


def job_spec() -> JobSpec:
    return JobSpec(
        executable=("/root/.local/bin/uv"),
        arguments=["run", "/slurm_mpi_worker/main.py"],
        name="test_job",
        directory=Path("/data"),
        stdout_path=Path("/data/logs.log"),
        stderr_path=Path("/data/errors.log"),
        resources=ResourceSpecV1(node_count=2, processes_per_node=1),
        attributes=JobAttributes(
            duration=timedelta(minutes=15),
            account="test_usr",
            custom_attributes={
                "slurm.open-mode": "append",
            },
        ),
        environment={
            "OMPI_ALLOW_RUN_AS_ROOT": 1,
            "OMPI_ALLOW_RUN_AS_ROOT_CONFIRM": 1,
        },
        launcher="mpirun",
    )


def test_psij_with_mpi() -> None:
    g = mpi_graph()
    storage = ControllerFileStorage(
        UUID(int=23),
        name="psij_mpi_graph",
        do_cleanup=True,
    )
    executor = PSIJExecutor(
        spec=job_spec(),
        launchers_path=None,
        logs_path=storage.logs_path,
        psij_executor=JobExecutor.get_instance("slurm"),
    )
    run_graph(storage, executor, g, {})

    output = read_outputs(g, storage)

    assert output is not None
    assert output == "Rank 0 out of 2 on c1.\nRank 1 out of 2 on c2."
