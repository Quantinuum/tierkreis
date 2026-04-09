# /// script
# requires-python = ">=3.12"
# dependencies = ["tierkreis", "mpi4py"]
# [tool.uv.sources]
# tierkreis = { path = "/tierkreis", editable = true }
# ///
import logging
import socket
from sys import argv

from mpi4py import MPI  # type: ignore

from tierkreis import Worker

logger = logging.getLogger(__name__)
worker = Worker("slurm_mpi_worker")

comm = MPI.COMM_WORLD


def _proc_info() -> dict[str, int | str]:
    """Returns a dictionary with the current process's details."""
    comm = MPI.COMM_WORLD
    rank = comm.Get_rank()
    hostname = socket.gethostname()
    return {"rank": rank, "hostname": hostname}


@worker.task()
def mpi_rank_info() -> str | None:
    comm = MPI.COMM_WORLD
    rank = comm.Get_rank()
    size = comm.Get_size()
    info = _proc_info()
    all_processes_info = comm.gather(info, root=0)
    if rank == 0:
        return "\n".join(
            f"Rank {info['rank']} out of {size} on {info['hostname']}."
            for info in all_processes_info
        )
    return None


@worker.task()
def mpi_fail() -> None:
    comm = MPI.COMM_WORLD
    rank = comm.Get_rank()
    msg = f"Process {rank} is failing now..."
    raise ValueError(msg)


if __name__ == "__main__":
    worker.app(argv)
