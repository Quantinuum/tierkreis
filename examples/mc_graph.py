from pathlib import Path
import random
from typing import NamedTuple
from uuid import UUID

from tierkreis.builder import GraphBuilder
from tierkreis.controller.data.core import EmptyModel
from tierkreis.controller.data.models import TKR
from tierkreis.executor import UvExecutor
from tierkreis.storage import FileStorage, read_outputs
from tierkreis import run_graph

from example_workers.mc_worker.stubs import ising_expected_stats, ising_mcmc_samples

import logging

# set up logger
logging.basicConfig(level=logging.INFO)
logger = logging.getLogger(__name__)
logger.info("mc_graph started")

class MCOutputs(NamedTuple):
    samples: TKR[list[str]]
    mean_energy: TKR[float]
    mean_magnetization: TKR[float]


def _ising_chain_J(n: int, coupling: float = 1.0) -> list[list[float]]:
    J = [[0.0 for _ in range(n)] for _ in range(n)]
    for i in range(n - 1):
        J[i][i + 1] = coupling
        J[i + 1][i] = coupling
    return J


g = GraphBuilder(EmptyModel, MCOutputs)

logger.info("mc_graph graph built")

# Example problem: 1D Ising chain with N spins, nearest-neighbour coupling.
N = 4
M = 200
seed = random.randint(0, 1000000)
temperature = 0.1

J = g.const(_ising_chain_J(N, coupling=0.6))
# h = g.const([0.2 if i % 2 == 0 else -0.2 for i in range(N)])
h = g.const([0.0 for _ in range(N)])

samples = g.task(ising_mcmc_samples(g.const(M), J, h, g.const(seed), g.const(temperature)))
stats = g.task(ising_expected_stats(samples, J, h))

g.outputs(MCOutputs(samples=samples, mean_energy=stats.mean_energy, mean_magnetization=stats.mean_magnetization))


if __name__ == "__main__":
    storage = FileStorage(UUID(int=210), do_cleanup=False, name="mc_graph_3")
    executor = UvExecutor(Path(__file__).parent / "example_workers", storage.logs_path)
    run_graph(storage, executor, g, {})
    outputs = read_outputs(g, storage)
    print(outputs)

