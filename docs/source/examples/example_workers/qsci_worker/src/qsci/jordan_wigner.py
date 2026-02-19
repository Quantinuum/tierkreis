import itertools
from collections import defaultdict

import numpy as np
from numpy.typing import NDArray
from pytket.circuit import Qubit
from pytket.pauli import Pauli, QubitPauliString

QubitHamiltonian = dict[QubitPauliString, float]


def jordan_wigner_one_body(i: int, j: int, coeff: float) -> QubitHamiltonian:
    r"""Map :math:`C a^\dagger_i a_j + \text{h.c.}` to qubits.

    Args:
        i: Index of the first orbital.
        j: Index of the second orbital.
        coeff: Coefficient of the term.

    Returns:
        List of tuples containing qubits, Pauli operators, and coefficients.
    """
    terms: QubitHamiltonian = defaultdict(float)

    if i != j:
        if i > j:
            i, j = j, i
            coeff = coeff.conjugate()

        parity_string = [(Qubit(p), Pauli.Z) for p in range(i + 1, j)]

        for c, (op_i, op_j) in [
            (coeff.real, (Pauli.X, Pauli.X)),
            (coeff.real, (Pauli.Y, Pauli.Y)),
            (-coeff.imag, (Pauli.X, Pauli.Y)),
            (coeff.imag, (Pauli.Y, Pauli.X)),
        ]:
            if c:
                strings = {
                    Qubit(i): op_i,
                    **dict(parity_string),
                    Qubit(j): op_j,
                }
                terms[QubitPauliString(strings)] += c * 0.5

    else:
        terms[QubitPauliString({})] += coeff * 0.5
        terms[QubitPauliString({Qubit(i): Pauli.Z})] += coeff * -0.5

    return terms


def jordan_wigner_two_body(
    i: int,
    j: int,
    k: int,
    l: int,  # noqa: E741
    coeff: float,
) -> QubitHamiltonian:
    r"""Map :math:`C a^\dagger_i a^\dagger_j a_k a_l + \text{h.c.}` to qubits.

    Args:
        i: Index of the first orbital.
        j: Index of the second orbital.
        k: Index of the third orbital.
        l: Index of the fourth orbital.
        coeff: Coefficient of the term.

    Returns:
        List of Pauli strings and coefficients.
    """
    terms: QubitHamiltonian = defaultdict(float)

    if (i == j) or (k == l):
        return terms

    elif len({i, j, k, l}) == 4:
        if (i > j) ^ (k > l):
            coeff *= -1

        for ops in itertools.product((Pauli.X, Pauli.Y), repeat=4):
            if ops.count(Pauli.X) % 2:
                c = coeff.imag * 0.125
                if ops in [
                    (Pauli.X, Pauli.Y, Pauli.X, Pauli.X),
                    (Pauli.Y, Pauli.X, Pauli.X, Pauli.X),
                    (Pauli.Y, Pauli.Y, Pauli.X, Pauli.Y),
                    (Pauli.Y, Pauli.Y, Pauli.Y, Pauli.X),
                ]:
                    c *= -1
            else:
                c = coeff.real * 0.125
                if ops not in [
                    (Pauli.X, Pauli.X, Pauli.Y, Pauli.Y),
                    (Pauli.Y, Pauli.Y, Pauli.X, Pauli.X),
                ]:
                    c *= -1

            if c:
                (ip, op_i), (jp, op_j), (kp, op_k), (lp, op_l) = sorted(
                    zip((i, j, k, l), ops)
                )
                parity_string_ij = [(Qubit(p), Pauli.Z) for p in range(ip + 1, jp)]
                parity_string_kl = [(Qubit(p), Pauli.Z) for p in range(kp + 1, lp)]
                strings = {
                    Qubit(ip): op_i,
                    **dict(parity_string_ij),
                    Qubit(jp): op_j,
                    Qubit(kp): op_k,
                    **dict(parity_string_kl),
                    Qubit(lp): op_l,
                }
                terms[QubitPauliString(strings)] += c

    elif len({i, j, k, l}) == 3:
        ip, jp, kp = 0, 0, 0
        if i == k:
            if j > l:
                ip, jp = l, j
                coeff = -coeff.conjugate()
            else:
                ip, jp = j, l
                coeff = -coeff
            kp = i
        elif i == l:
            if j > k:
                ip, jp = k, j
                coeff = coeff.conjugate()
            else:
                ip, jp = j, k
            kp = i
        elif j == k:
            if i > l:
                ip, jp = l, i
                coeff = coeff.conjugate()
            else:
                ip, jp = i, l
            kp = j
        elif j == l:
            if i > k:
                ip, jp = k, i
                coeff = -coeff.conjugate()
            else:
                ip, jp = i, k
                coeff = -coeff
            kp = j

        parity_string = [(Qubit(p), Pauli.Z) for p in range(ip + 1, jp)]

        for c, (op_i, op_j) in [
            (coeff.real, (Pauli.X, Pauli.X)),
            (coeff.real, (Pauli.Y, Pauli.Y)),
            (-coeff.imag, (Pauli.X, Pauli.Y)),
            (coeff.imag, (Pauli.Y, Pauli.X)),
        ]:
            if c:
                strings = {
                    Qubit(ip): op_i,
                    **dict(parity_string),
                    Qubit(jp): op_j,
                }
                terms[QubitPauliString(strings)] += c * 0.25

                # TODO: Check this
                strings = strings.copy()
                if Qubit(kp) in strings:
                    del strings[Qubit(kp)]
                else:
                    strings[Qubit(kp)] = Pauli.Z
                terms[QubitPauliString(strings)] += c * -0.25

    elif len({i, j, k, l}) == 2:
        if i == l:
            c = coeff * -0.25
        else:
            c = coeff * 0.25
        ip, jp = sorted([i, j])

        terms[QubitPauliString({})] += -c
        terms[QubitPauliString({Qubit(ip): Pauli.Z})] += c
        terms[QubitPauliString({Qubit(jp): Pauli.Z})] += c
        terms[QubitPauliString({Qubit(ip): Pauli.Z, Qubit(jp): Pauli.Z})] += -c

    return terms


def _apply_threshold(hamiltonian: QubitHamiltonian, tol: float) -> QubitHamiltonian:
    """Remove terms with coefficients below a threshold.

    Args:
        hamiltonian: Dictionary of Pauli strings and coefficients.
        tol: Threshold value.

    Returns:
        Dictionary of Pauli strings and coefficients.
    """
    return {
        strings: coeff for strings, coeff in hamiltonian.items() if np.abs(coeff) > tol
    }


def qubit_mapping_jordan_wigner(
    h0: float, h1: NDArray[np.inexact], h2: NDArray[np.inexact], tol: float = 1e-12
) -> QubitHamiltonian:
    """Map the Hamiltonian to qubits using Jordan--Wigner mapping.

    Args:
        h0: Constant term.
        h1: One-body Hamiltonian.
        h2: Two-body Hamiltonian.

    Returns:
        Dictionary of Pauli strings and coefficients.
    """
    norb = h1.shape[0]
    hamiltonian: QubitHamiltonian = defaultdict(float)

    def _update_hamiltonian(terms: QubitHamiltonian) -> None:
        for strings, coeff in terms.items():
            hamiltonian[strings] += coeff

    # Zero-body term
    hamiltonian[QubitPauliString({})] = h0

    # One-body terms
    for i, j in itertools.combinations_with_replacement(range(norb), r=2):
        _update_hamiltonian(jordan_wigner_one_body(i, j, h1[i, j]))

    # Two-body terms
    for (i, j), (k, l) in itertools.combinations_with_replacement(  # noqa: E741
        itertools.combinations_with_replacement(range(norb), r=2), r=2
    ):
        _update_hamiltonian(jordan_wigner_two_body(i, j, l, k, h2[i, j, k, l]))

    # Remove zero terms
    hamiltonian = _apply_threshold(hamiltonian, tol)

    return dict(hamiltonian)
