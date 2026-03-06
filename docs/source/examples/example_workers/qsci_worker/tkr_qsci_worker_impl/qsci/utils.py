import itertools
from typing import cast

import numpy as np
from numpy.typing import NDArray
from pytket._tket.circuit import Circuit
from pytket.circuit import PauliExpBox
from pytket.pauli import Pauli
from pytket.utils.operators import QubitPauliOperator

from .active_space import get_n_active, get_n_core, get_n_virtual


def get_configs(
    nele: int,
    norb: int,
    mult: int,
) -> list[tuple[int, ...]]:
    """Get the spin- and particle-number-conserving configurations."""
    refc_a = [0 for _ in range(norb)]
    refc_b = [0 for _ in range(norb)]
    nele_a = (nele + mult - 1) // 2
    nele_b = (nele - mult + 1) // 2
    for i in range(nele_a):
        refc_a[i] = 1
    for i in range(nele_b):
        refc_b[i] = 1
    ls = []
    lsp = []
    for pj in itertools.permutations(refc_b):
        if pj not in lsp:
            lsp.append(pj)
    for pj in lsp:
        for pi in lsp:
            p = []
            for i, j in zip(pi, pj, strict=True):
                p += [i, j]
            p = tuple(p)
            if p not in ls:
                ls.append(p)
    return ls


def get_config_from_cas_init(
    mo_occ: list[int],
    n_cas_init: int,
    n_elecas_init: int,
    n_cas_hsim: int,
    n_elecas_hsim: int,
) -> list[tuple[int, ...]]:
    """Get configurations (SD) in the H simulation space."""
    nele = sum(mo_occ) - 2 * get_n_core(mo_occ, n_elecas_init)
    norb = get_n_active(mo_occ, n_cas_init, n_elecas_init)
    phis_init = get_configs(
        nele=nele,
        norb=norb,
        mult=1,
    )
    n_core = get_n_core(mo_occ, n_elecas_init)
    n_core -= get_n_core(mo_occ, n_elecas_hsim)
    n_virt = get_n_virtual(mo_occ, n_cas_init, n_elecas_init)
    n_virt -= get_n_virtual(mo_occ, n_cas_hsim, n_elecas_hsim)
    lsdoc = [1 for _ in range(2 * n_core)]
    lsvir = [0 for _ in range(2 * n_virt)]
    return [tuple(lsdoc + list(i) + lsvir) for i in phis_init]


def make_time_evolution_circuits(
    t_step_list: list[float],
    prepared_circuit: Circuit,
    h_hsim: QubitPauliOperator,
    h_init: QubitPauliOperator,
    max_cx_gates: int,
) -> list[Circuit]:
    """Time-evolution of the prepared circuit.

    Args:
        t_step_list: List of time step sizes.
        prepared_circuit: Circuit.
        h_hsim: Hamiltonian in the space for time-evolution.
        h_init: Hamiltonian in the space for the state preparation.
        max_cx_gates: Number of maximum CX gates to be included.

    Returns:
        List of time-evolution circuits.
    """
    list_circ: list[Circuit] = []
    n_trotter = 1

    H_for_time_evolution = QubitPauliOperator(
        {qps: h_hsim[qps] - h_init.get(qps, 0.0) for qps in h_hsim._dict},
    )
    H_for_time_evolution.compress()
    items = sorted(
        H_for_time_evolution._dict.items(),
        key=lambda x: abs(x[1]),
        reverse=True,
    )
    for time_step in t_step_list:
        circ = prepared_circuit.copy()
        # NOTE: Need more trotter steps?
        for _ in range(n_trotter):
            n_cx_gates = 0
            for pauli_string, coeff in items:
                # Check the number of CX to be added.
                n_cx_next = len(pauli_string.map.keys())
                if n_cx_next > 0:
                    n_cx_gates += 2 * (n_cx_next - 1)
                if n_cx_gates > max_cx_gates:
                    break
                ls = [Pauli.I for _ in range(len(circ.qubits))]
                for q, p in pauli_string.map.items():
                    ls[q.index[0]] = p
                if any(p != Pauli.I for p in ls):
                    circ.add_pauliexpbox(
                        PauliExpBox(ls, coeff * 2 * time_step / np.pi),
                        list(range(len(circ.qubits))),
                    )
        circ.measure_all()
        list_circ.append(circ)
    return list_circ


def rhf2ghf(
    h0: float,
    h1e0: NDArray[np.float64],
    h2e0: NDArray[np.float64],
) -> tuple[float, NDArray[np.float64], NDArray[np.float64]]:
    """Transform RHF integrals into the GHF basis.

    Args:
        h0: constant.
        h1e0: 1e integrals in the RHF basis
        h2e0: 2e integrals in the RHF basis

    Returns:
        Integrals in the GHF basis.
    """
    nmo = h1e0.shape[0]
    h1e = cast("NDArray[np.float64]", np.kron(np.eye(2), h1e0))
    h2e = np.kron(np.eye(2), np.kron(np.eye(2), h2e0).T)
    mask = list(itertools.chain(*zip(range(nmo), range(nmo, nmo * 2), strict=True)))
    h1e = h1e[mask][:, mask]
    h2e = h2e[mask][:, mask][:, :, mask][:, :, :, mask]
    h2e = h2e.transpose(0, 2, 1, 3) - h2e.transpose(0, 2, 3, 1)
    return h0, h1e, h2e
