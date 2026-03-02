import logging
from collections import Counter
from sys import argv
from typing import NamedTuple, cast

import numpy as np
from pytket._tket.circuit import Circuit
from pytket.backends.backendresult import BackendResult
from pytket.circuit import Qubit
from pytket.pauli import QubitPauliString
from pytket.utils.operators import CoeffTypeAccepted, QubitPauliOperator

from tierkreis import Worker

from .chemistry.active_space import get_frozen
from .chemistry.molecule import extract_hamiltonian_rhf
from .qsci.active_space import get_n_active, get_n_core
from .qsci.jordan_wigner import qubit_mapping_jordan_wigner
from .qsci.postprocess import get_ci_matrix, postprocess_configs
from .qsci.state_prep import perform_state_preparation
from .qsci.utils import get_config_from_cas_init, make_time_evolution_circuits, rhf2ghf

logger = logging.getLogger(__name__)

worker = Worker("qsci_worker")


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
def state_prep(
    ham_init: Hamiltonian,
    reference_state: list[int],
    max_iteration_prep: int,
    atol: float,
    mo_occ: list[int],
    cas_init: CompleteActiveSpace,
    cas_hsim: CompleteActiveSpace,
) -> Circuit:
    ham_init_operator = QubitPauliOperator(
        cast(
            "dict[QubitPauliString, CoeffTypeAccepted]",
            qubit_mapping_jordan_wigner(
                *rhf2ghf(
                    ham_init.h0,
                    np.array(ham_init.h1),
                    np.array(ham_init.h2),
                ),
            ),
        ),
    )
    # time-evolve CASCI ground state.
    n_core_init = get_n_core(mo_occ, cas_init.n_ele)
    n_core_hsim = get_n_core(mo_occ, cas_hsim.n_ele)
    n_core = n_core_init - n_core_hsim
    logging.info(
        f"mo_occ={mo_occ} n_cas_hsim={cas_hsim.n} n_elecas_hsim={cas_hsim.n_ele}",
    )
    n_active_hsim = get_n_active(mo_occ, cas_hsim.n, cas_hsim.n_ele)
    prepared_circ = Circuit(n_active_hsim * 2)
    for i in range(n_core * 2):
        prepared_circ.X(i)
    return perform_state_preparation(
        reference_state=reference_state,
        ham_init=ham_init_operator,
        n_cas_init=cas_init.n,
        max_iteration=max_iteration_prep,
        atol=atol,
    )


@worker.task()
def circuits_from_hamiltonians(
    ham_init: Hamiltonian,
    ham_hsim: Hamiltonian,
    adapt_circuit: Circuit,
    t_step_list: list[float],
    cas_init: CompleteActiveSpace,
    cas_hsim: CompleteActiveSpace,
    mo_occ: list[int],
    max_cx_gates: int,
) -> list[Circuit]:
    ham_init_operator = QubitPauliOperator(
        cast(
            "dict[QubitPauliString, CoeffTypeAccepted]",
            qubit_mapping_jordan_wigner(
                *rhf2ghf(
                    ham_init.h0,
                    np.array(ham_init.h1),
                    np.array(ham_init.h2),
                ),
            ),
        ),
    )
    ham_hsim_operator = QubitPauliOperator(
        cast(
            "dict[QubitPauliString, CoeffTypeAccepted]",
            qubit_mapping_jordan_wigner(
                *rhf2ghf(
                    ham_hsim.h0,
                    np.array(ham_hsim.h1),
                    np.array(ham_hsim.h2),
                ),
            ),
        ),
    )
    # Load the input data.
    n_core_init = get_n_core(mo_occ, cas_init.n_ele)
    n_core_hsim = get_n_core(mo_occ, cas_hsim.n_ele)
    n_core = n_core_init - n_core_hsim
    n_active_hsim = get_n_active(mo_occ, cas_hsim.n, cas_hsim.n_ele)
    prepared_circ = Circuit(n_active_hsim * 2)
    for i in range(n_core * 2):
        prepared_circ.X(i)
    prepared_circ.add_circuit(
        adapt_circuit,
        [2 * n_core + i for i in range(adapt_circuit.n_qubits)],
    )
    ham_init_shifted = QubitPauliOperator(
        {
            QubitPauliString(
                {
                    Qubit(qubit.index[0] + 2 * n_core): pauli
                    for qubit, pauli in qps.map.items()
                },
            ): coeff
            for qps, coeff in ham_init_operator._dict.items()
        },
    )
    return make_time_evolution_circuits(
        t_step_list,
        prepared_circ,
        h_hsim=ham_hsim_operator,
        h_init=ham_init_shifted,
        max_cx_gates=max_cx_gates,
    )


@worker.task()
def energy_from_results(
    ham_hsim: Hamiltonian,
    backend_results: list[BackendResult],
    mo_occ: list[int],
    cas_init: CompleteActiveSpace,
    cas_hsim: CompleteActiveSpace,
) -> float:
    counts = Counter()
    for r in backend_results:
        for k, v in r.get_counts().items():
            counts[k] += v
    phis = list(counts.keys())
    phis_init_orig = get_config_from_cas_init(
        mo_occ,
        cas_init.n,
        cas_init.n_ele,
        cas_hsim.n,
        cas_hsim.n_ele,
    )
    for p in phis_init_orig:
        if p not in phis:
            phis.append(p)
    # phis = get_config(backend_results)
    logger.info(f"CONFIG (before): {len(phis)}")
    phis = postprocess_configs(phis_init_orig[0], phis)
    logger.info(f"CONFIG (after):  {len(phis)}")

    hsd = get_ci_matrix(
        phis,
        h1=np.array(ham_hsim.h1),
        h2=np.array(ham_hsim.h2),
        enuc=ham_hsim.h0,
    )
    energy = np.linalg.eigh(hsd.todense())[0][0]
    logger.info(f"ENERGY: {energy}")

    return energy


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
