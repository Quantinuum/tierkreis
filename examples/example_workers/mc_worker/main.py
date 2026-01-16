# /// script
# requires-python = ">=3.12"
# dependencies = ["tierkreis"]
#
# [tool.uv.sources]
# tierkreis = { path = "../../../tierkreis", editable = true }
# ///

import math
import random
from sys import argv
from typing import NamedTuple
from logging import getLogger

from tierkreis import Worker
from tierkreis.controller.data.models import portmapping

worker = Worker("mc_worker")
logger = getLogger(__name__)


def _validate_ising_params(J: list[list[float]], h: list[float]) -> int:
    n = len(h)
    if n == 0:
        raise ValueError("h must be non-empty")
    if len(J) != n or any(len(row) != n for row in J):
        raise ValueError(f"J must be a {n}x{n} matrix to match h (len={n})")
    return n


def _bitstring_from_spins(spins: list[int]) -> str:
    # spins are in {-1, +1}; encode as bits {0,1}
    return "".join("1" if s > 0 else "0" for s in spins)


def _spins_from_bitstring(bits: str) -> list[int]:
    return [1 if c == "1" else -1 for c in bits]


@worker.task()
def ising_mcmc_samples(
    M: int,
    J: list[list[float]],
    h: list[float],
    seed: int,
    temperature: float = 1.0,
) -> list[str]:
    """Metropolis MCMC for an Ising model at temperature T (kB=1), returning M bitstrings.

    - **J**: NxN coupling matrix (use upper triangle / symmetric in practice)
    - **h**: N local fields
    - **temperature**: T > 0 (kB=1)
    - **Output**: list of length M, each a '0'/'1' string of length N
    """
    if M < 0:
        raise ValueError("M must be >= 0")
    if temperature <= 0:
        raise ValueError("temperature must be > 0")
    n = _validate_ising_params(J, h)

    logger.info(
        "ising_mcmc_samples started (M=%s, N=%s, seed=%s, temperature=%s)", M, n, seed, temperature
    )
    rng = random.Random(seed)
    spins: list[int] = [1 if rng.random() < 0.5 else -1 for _ in range(n)]

    burn_in_sweeps = max(50, 10 * n)
    sweeps_per_sample = max(1, n // 2)
    logger.info(
        "ising_mcmc_samples config (burn_in_sweeps=%s, sweeps_per_sample=%s)",
        burn_in_sweeps,
        sweeps_per_sample,
    )

    def local_field(i: int) -> float:
        return h[i] + sum(J[i][j] * spins[j] for j in range(n) if j != i)

    accepted = 0
    attempted = 0

    def sweep() -> None:
        nonlocal accepted, attempted
        for i in range(n):
            attempted += 1
            # ΔE for flipping s_i: 2*s_i*(h_i + Σ_j J_ij s_j)
            lf = local_field(i)
            dE = 2.0 * spins[i] * lf
            if dE <= 0.0 or rng.random() < math.exp(-dE / temperature):
                spins[i] *= -1
                accepted += 1

    for _ in range(burn_in_sweeps):
        sweep()
    if attempted:
        logger.info(
            "ising_mcmc_samples finished burn-in (acceptance_rate=%.3f)",
            accepted / attempted,
        )

    samples: list[str] = []
    progress_every = max(1, M // 5) if M else 0  # log ~5 times at most
    for i in range(M):
        for _ in range(sweeps_per_sample):
            sweep()
        samples.append(_bitstring_from_spins(spins))
        if progress_every and (i + 1) % progress_every == 0:
            logger.info("ising_mcmc_samples progress (%s/%s samples)", i + 1, M)

    if attempted:
        logger.info(
            "ising_mcmc_samples finished (samples=%s, acceptance_rate=%.3f)",
            len(samples),
            accepted / attempted,
        )
    return samples


@portmapping
class IsingStats(NamedTuple):
    mean_energy: float
    mean_magnetization: float


@worker.task()
def ising_expected_stats(
    samples: list[str], J: list[list[float]], h: list[float]
) -> IsingStats:
    """Compute expectations from bitstring samples under E(s) = -Σ_{i<j} J_ij s_i s_j - Σ_i h_i s_i."""
    n = _validate_ising_params(J, h)
    if len(samples) == 0:
        raise ValueError("samples must be non-empty")

    def energy(spins: list[int]) -> float:
        e = 0.0
        for i in range(n):
            e -= h[i] * spins[i]
        for i in range(n):
            for j in range(i + 1, n):
                e -= J[i][j] * spins[i] * spins[j]
        return e

    logger.info("ising_expected_stats started")
    total_e = 0.0
    total_m = 0.0
    for b in samples:
        if len(b) != n:
            raise ValueError(f"sample bitstring length {len(b)} != N {n}")
        s = _spins_from_bitstring(b)
        total_e += energy(s)
        total_m += sum(s) / n

    inv = 1.0 / len(samples)
    return IsingStats(mean_energy=total_e * inv, mean_magnetization=total_m * inv)


if __name__ == "__main__":
    worker.app(argv)

