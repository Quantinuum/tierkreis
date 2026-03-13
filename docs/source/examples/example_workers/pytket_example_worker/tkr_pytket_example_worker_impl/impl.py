import logging

from pytket._tket.circuit import Circuit
from pytket.backends.backendresult import BackendResult
from pytket.extensions.qiskit import AerBackend
from pytket.transform import Transform
from sympy import Symbol

from tierkreis import Worker

logger = logging.getLogger(__name__)

worker = Worker("pytket_example_worker")


@worker.task()
def substitute(circuit: Circuit, a: float, b: float, c: float) -> Circuit:
    circuit.symbol_substitution({Symbol("a"): a, Symbol("b"): b, Symbol("c"): c})
    return circuit


@worker.task()
def simulate(circuit: Circuit) -> BackendResult:
    backend = AerBackend()
    return backend.run_circuit(circuit, n_shots=1000)


@worker.task()
def optimise(circuit: Circuit) -> Circuit:
    Transform.OptimisePhaseGadgets().apply(circuit)
    return circuit
