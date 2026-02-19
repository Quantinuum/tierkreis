import itertools

import numpy as np
from numpy.typing import NDArray
from pytket._tket.circuit import Circuit
from pytket.circuit import PauliExpBox, Qubit
from pytket.extensions.qiskit.backends.aer import AerStateBackend
from pytket.pauli import Pauli, QubitPauliString
from pytket.utils.operators import CoeffTypeAccepted, QubitPauliOperator
from scipy.optimize import minimize
from sympy import Symbol


def make_pool(qubit_number: int) -> list[QubitPauliOperator]:
    """Make operator pool for state preparation.

    Args:
        qubit_number: number of qubits.

    Returns:
        List of excitation operators.

    Note:
        It doesn't assume the particle number conserving in the JW mapping.
    """
    pool: list[QubitPauliOperator] = []
    # First order:
    for i, k in itertools.combinations(range(qubit_number), 2):
        terms: dict[QubitPauliString, CoeffTypeAccepted] = {
            QubitPauliString({Qubit(i): Pauli.X, Qubit(k): Pauli.Y}): -0.5j,
            QubitPauliString({Qubit(i): Pauli.Y, Qubit(k): Pauli.X}): 0.5j,
        }
        pool.append(QubitPauliOperator(terms))
    # Second order:
    for i, j, k, l in itertools.combinations(range(qubit_number), 4):  # noqa: E741
        terms1 = {}
        terms2 = {}
        terms3 = {}
        for paulis, factor in [
            ("XYXX", 0.125j),
            ("YXXX", 0.125j),
            ("YYYX", 0.125j),
            ("YYXY", 0.125j),
            ("XXYX", -0.125j),
            ("XXXY", -0.125j),
            ("XYYY", -0.125j),
            ("YXYY", -0.125j),
        ]:
            qps = QubitPauliString(
                {Qubit(x): getattr(Pauli, p) for x, p in zip([i, j, k, l], paulis)}
            )
            terms1[qps] = factor
            qps = QubitPauliString(
                {Qubit(x): getattr(Pauli, p) for x, p in zip([i, k, j, l], paulis)}
            )
            terms2[qps] = factor
            qps = QubitPauliString(
                {Qubit(x): getattr(Pauli, p) for x, p in zip([i, l, j, k], paulis)}
            )
            terms3[qps] = factor
        pool.append(QubitPauliOperator(terms1))
        pool.append(QubitPauliOperator(terms2))
        pool.append(QubitPauliOperator(terms3))
    return pool


def costfunc(
    params: list[float],
    ansatz: Circuit,
    target: np.ndarray,
) -> float:
    """Cost function for ADAPTive state preparation

    Args:
        params: Circuit parameters
        ansatz: Ansatz circuit
        target: Target statevector

    Returns:
        cost function value
    """
    circ_copy = ansatz.copy()
    symbols = circ_copy.free_symbols()
    ls = list(symbols)
    mapping = dict(zip(ls, params))
    circ_copy.symbol_substitution(mapping)
    backend = AerStateBackend()
    compiled_circ = backend.get_compiled_circuit(circ_copy, optimisation_level=0)
    result = backend.run_circuit(compiled_circ)
    adapt_circuit_vector = result.get_state()
    s2 = np.sum(np.abs(adapt_circuit_vector.dot(target)) ** 2)
    return 1.0 - s2


def state_preparation(
    reference: Circuit,
    pool: list[QubitPauliOperator],
    target: NDArray[np.float64],
    max_iteration: int,
    atol: float = 0.01,
    strict: bool = False,
) -> Circuit:
    """State preparation with the ADAPT optimization on classical computers.

    Args:
        reference: Reference circuit from which ADAPT starts.
        pool: List of excitation oeprators.
        target: Target state-vector to be approximated.
        max_iteration: Maximum number of iteration.
        atol: Absolute tolerance.
        strict: Raise exception if True when not converge.
    Returns:
        Circuit to approximately reproduce the
    """
    # Generate the reference state circuit.
    adapt_circ = reference.copy()
    ref_statevector: NDArray[np.complex128] = adapt_circ.get_statevector()
    cost = costfunc([], adapt_circ, target)
    # Pre-compute the matrices corresponding to the excitation operators in the pool.
    pool_matrices = [
        p.to_sparse_matrix(qubits=adapt_circ.qubits).todense() for p in pool
    ]
    # Start ADAPT iterative optimization.
    x0: list[float] = []
    for iteration in range(max_iteration):
        if cost < atol:
            break
        ls_s2: list[float] = []
        for a_matrix in pool_matrices:
            val: float = np.vdot(target, a_matrix.dot(ref_statevector)).real
            ls_s2.append(val)
        max_index = np.argmax(ls_s2)
        # Optimize the parameterized quantum circuit.
        s = "t" + str(iteration)  ## circuit parameter
        for pauli_string, coeff in pool[max_index]._dict.items():
            ls = [Pauli.I for _ in range(reference.n_qubits)]
            for q, p in pauli_string.map.items():
                ls[q.index[0]] = p
            if any([p != Pauli.I for p in ls]):
                adapt_circ.add_pauliexpbox(
                    PauliExpBox(
                        ls,
                        complex(coeff).imag * np.real(Symbol(s)) * 2.0 / np.pi,  # type: ignore
                    ),
                    [j for j in range(reference.n_qubits)],
                )
        x0_array = 1 - 2 * np.random.random(size=iteration + 1)
        opt_res = minimize(costfunc, x0_array, args=(adapt_circ, target))
        # Update the reference state-vector and repeat the ADAPT procedure.
        x0 = opt_res.x.tolist()
        mapping = dict(zip(adapt_circ.free_symbols(), opt_res.x))
        circ = adapt_circ.copy()
        circ.symbol_substitution(mapping)
        ref_statevector = circ.get_statevector()
        cost = opt_res.fun
        print("error after iteration=" + str(iteration) + ":", opt_res.fun, opt_res.x)
    else:
        if strict:
            raise RuntimeError("Not converge")
        else:
            print("Not converge")
    symbols = adapt_circ.free_symbols()
    mapping = dict(zip(symbols, x0))
    adapt_circ.symbol_substitution(mapping)
    return adapt_circ


def perform_state_preparation(
    reference_state: list[int],
    ham_init: QubitPauliOperator,
    n_cas_init: int,
    max_iteration: int,
    atol: float,
) -> Circuit:
    """State preparation or load in the saved one."""
    adapt_circuit = Circuit(len(reference_state))
    target_vector: NDArray[np.complex128] = np.linalg.eigh(
        ham_init.to_sparse_matrix().todense()
    )[1][:, 0]
    target_vector_reshaped = np.array(target_vector).reshape(-1).real

    if n_cas_init > 0:
        # Define operator pool and convert the CASCI ground state into circuit
        qubit_number = len(ham_init.all_qubits)
        pool = make_pool(qubit_number)
        for i, x in enumerate(reference_state):
            if x:
                adapt_circuit.X(i)
        adapt_circuit = state_preparation(
            reference=adapt_circuit,
            pool=pool,
            target=target_vector_reshaped,
            max_iteration=max_iteration,
            atol=atol,
        )
    return adapt_circuit
