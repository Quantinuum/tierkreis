from uuid import UUID

import pytest

from tests.executor.samples import (
    mpi_fail_graph,
    mpi_graph,
    pbs_spec,
    slurm_spec,
)
from tierkreis.controller import run_graph
from tierkreis.controller.executor.hpc.psij_executor import PSIJExecutor
from tierkreis.controller.storage.filestorage import ControllerFileStorage
from tierkreis.exceptions import TierkreisError
from tierkreis.storage import read_outputs


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
