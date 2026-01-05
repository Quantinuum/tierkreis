# /// script
# requires-python = ">=3.12"
# dependencies = ["pydantic", "tierkreis", "pyscf", "ruff"]
#
# [tool.uv.sources]
# tierkreis = { path = "../../../tierkreis", editable = true }
# ///
import logging
from sys import argv
from typing import NamedTuple


from chemistry.active_space import get_frozen
from chemistry.molecule import extract_hamiltonian_rhf

from tierkreis import Worker

logger = logging.getLogger(__name__)

worker = Worker("chemistry_worker")


class Molecule(NamedTuple):
    geometry: list[tuple[str, list[float]]]
    basis: str
    charge: int


class CompleteActiveSpace(NamedTuple):
    n: int
    n_ele: int


class Hamiltonian(NamedTuple):
    h0: float
    h1: list[list[float]]
    h2: list[list[list[list[float]]]]


@worker.task()
def make_ham(
    molecule: Molecule,
    mo_occ: list[int],
    cas: CompleteActiveSpace,
) -> Hamiltonian:
    # Construct the frozen orbital lists.
    frozen = get_frozen(mo_occ, cas.n, cas.n_ele)
    h0, h1, h2 = extract_hamiltonian_rhf(
        molecule.geometry,
        molecule.basis,
        charge=molecule.charge,
        frozen=frozen,
    )
    return Hamiltonian(h0=h0, h1=h1.tolist(), h2=h2.tolist())


if __name__ == "__main__":
    worker.app(argv)
