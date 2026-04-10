from datetime import timedelta
from uuid import UUID

from psij import JobAttributes, ResourceSpecV1
from psij import JobSpec as PSIJJobSpec
from tests.executor.stubs import mpi_rank_info
from tierkreis.builder import Graph
from tierkreis.controller import run_graph
from tierkreis.controller.data.graph import GraphData
from tierkreis.controller.data.models import TKR
from tierkreis.controller.executor.hpc.job_spec import (
    JobSpec as TKRJobSpec,
)
from tierkreis.controller.executor.hpc.job_spec import (
    MpiSpec,
    ResourceSpec,
)
from tierkreis.controller.executor.hpc.pbs import PBSExecutor
from tierkreis.controller.storage.filestorage import ControllerFileStorage
from tierkreis.storage import read_outputs


def mpi_graph() -> GraphData:
    builder = Graph(outputs_type=TKR[str | None])
    mpi_result = builder.task(mpi_rank_info())
    builder.finish_with_outputs(mpi_result)
    return builder.data


def tkr_job_spec() -> TKRJobSpec:
    return TKRJobSpec(
        job_name="test_job",
        account="test_usr",
        command=("/root/.local/bin/uv run /slurm_mpi_worker/main.py "),
        resource=ResourceSpec(nodes=2, memory_gb=None),
        walltime="00:15:00",
        mpi=MpiSpec(max_proc_per_node=1),
    )


def psji_job_spec() -> PSIJJobSpec:
    return PSIJJobSpec(
        executable=("/root/.local/bin/uv"),
        arguments=["run", "/slurm_mpi_worker/main.py"],
        name="test_job",
        resources=ResourceSpecV1(node_count=2, processes_per_node=1),
        attributes=JobAttributes(
            duration=timedelta(minutes=15),
            account="test_usr",
        ),
        launcher="mpirun",
    )


def main() -> None:
    g = mpi_graph()
    storage = ControllerFileStorage(
        UUID(int=22),
        name="mpi_graph",
        do_cleanup=True,
    )
    tkr_executor = PBSExecutor(
        spec=tkr_job_spec(),
        registry_path=None,
        logs_path=storage.logs_path,
    )
    # psij_executor = PSIJExecutor(
    #     spec=psji_job_spec(),
    #     launchers_path=None,
    #     logs_path=storage.logs_path,
    #     psij_executor=JobExecutor.get_instance("pbs"),
    # )

    run_graph(storage, tkr_executor, g, {})

    output = read_outputs(g, storage)
    print(output)
    assert output is not None
    assert output == "Rank 0 out of 2 on c1.\nRank 1 out of 2 on c2."
