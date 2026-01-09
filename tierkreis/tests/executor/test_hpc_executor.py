from pathlib import Path
from uuid import UUID
import pytest
from tierkreis.builder import GraphBuilder
from tierkreis.controller import run_graph
from tierkreis.controller.data.models import TKR
from tierkreis.controller.executor.hpc.job_spec import (
    JobSpec,
    MpiSpec,
    ResourceSpec,
)
from tierkreis.controller.executor.hpc.slurm import SLURMExecutor
from tierkreis.controller.storage.filestorage import ControllerFileStorage
from tierkreis_core import GraphData

from tests.executor.stubs import mpi_rank_info
from tierkreis.storage import read_outputs


def mpi_graph() -> GraphData:
    builder = GraphBuilder(outputs_type=TKR[str | None])
    mpi_result = builder.task(mpi_rank_info())
    builder.outputs(mpi_result)
    return builder.data


def job_spec() -> JobSpec:
    return JobSpec(
        job_name="test_job",
        account="test_usr",
        command="--allow-run-as-root /root/.local/bin/uv run /slurm_mpi_worker/main.py ",
        resource=ResourceSpec(nodes=2, memory_gb=None),
        walltime="00:15:00",
        mpi=MpiSpec(max_proc_per_node=1),
        extra_scheduler_args={"--open-mode=append": None},
        output_path=Path("./logs.log"),
        error_path=Path("./errors.log"),
    )


@pytest.mark.skip(reason="Needs SLURM setup.")
def test_slurm_with_mpi() -> None:
    g = mpi_graph()
    storage = ControllerFileStorage(
        UUID(int=22),
        name="mpi_graph",
        do_cleanup=True,
    )
    sbatch = str(
        Path(__file__).parent.parent.parent.parent / "infra/slurm_local/sbatch"
    )
    executor = SLURMExecutor(
        spec=job_spec(),
        registry_path=None,
        logs_path=storage.logs_path,
        command=sbatch,
    )
    run_graph(storage, executor, g, {})

    output = read_outputs(g, storage)

    assert output is not None
    assert output == "Rank 0 out of 2 on c1.\nRank 1 out of 2 on c2."
