from pathlib import Path
from uuid import UUID

import pytest

from tests.executor.samples import (
    mpi_graph,
    pbs_spec,
    slurm_spec,
)
from tierkreis.controller import run_graph
from tierkreis.controller.executor.hpc.pbs import PBSExecutor
from tierkreis.controller.executor.hpc.slurm import SLURMExecutor
from tierkreis.controller.storage.filestorage import ControllerFileStorage
from tierkreis.storage import read_outputs


@pytest.mark.skip(reason="Needs PBS setup.")
def test_pbs_with_mpi() -> None:
    g = mpi_graph()
    storage = ControllerFileStorage(
        UUID(int=24),
        name="mpi_graph_pbs",
        do_cleanup=True,
    )
    qsub = str(
        Path(__file__).parent.parent.parent.parent / "infra/pbs_local/qsub",
    )
    executor = PBSExecutor(
        spec=pbs_spec(),
        registry_path=None,
        logs_path=storage.logs_path,
        command=qsub,
    )
    run_graph(storage, executor, g, {})

    output = read_outputs(g, storage)

    assert output is not None
    assert output == "Rank 0 out of 2 on p1.\nRank 1 out of 2 on p2."


@pytest.mark.skip(reason="Needs SLURM setup.")
def test_slurm_with_mpi() -> None:
    g = mpi_graph()
    storage = ControllerFileStorage(
        UUID(int=22),
        name="mpi_graph_slurm",
        do_cleanup=True,
    )
    sbatch = str(
        Path(__file__).parent.parent.parent.parent / "infra/slurm_local/sbatch",
    )
    executor = SLURMExecutor(
        spec=slurm_spec(),
        registry_path=None,
        logs_path=storage.logs_path,
        command=sbatch,
    )
    run_graph(storage, executor, g, {})

    output = read_outputs(g, storage)

    assert output is not None
    assert output == "Rank 0 out of 2 on c1.\nRank 1 out of 2 on c2."
