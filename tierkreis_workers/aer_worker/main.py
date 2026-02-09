import logging
from sys import argv
from typing import Any

from tierkreis import Worker
from pytket._tket.circuit import Circuit
from pytket.backends.backendresult import BackendResult
from pytket.extensions.qiskit.qiskit_convert import tk_to_qiskit
from pytket.extensions.qiskit.backends.aer import AerBackend
from qiskit import qasm3

worker = Worker("aer_worker")
logger = logging.getLogger(__name__)


def get_backend(simulation_method: str = "automatic", n_qubits: int = 40) -> AerBackend:
    return AerBackend(simulation_method=simulation_method, n_qubits=n_qubits)


def get_config(seed: int | None = None) -> dict[str, Any]:
    return {} if seed is None else {"seed": seed}


@worker.task()
def get_compiled_circuit(
    circuit: Circuit,
    optimisation_level: int = 2,
    timeout: int = 300,
    simulation_method: str = "automatic",
    n_qubits: int = 40,
) -> Circuit:
    logger.info("get_compiled_circuit")

    backend = get_backend(simulation_method, n_qubits)
    return backend.get_compiled_circuit(circuit, optimisation_level, timeout)


@worker.task()
def run_circuit(
    circuit: Circuit,
    n_shots: int,
    simulation_method: str = "automatic",
    n_qubits: int = 40,
    seed: int | None = None,
) -> BackendResult:
    logger.info("run_circuit")
    backend = get_backend(simulation_method, n_qubits)
    config = get_config(seed)
    return backend.run_circuit(circuit, n_shots, **config)


@worker.task()
def run_circuits(
    circuits: list[Circuit],
    n_shots: list[int],
    simulation_method: str = "automatic",
    n_qubits: int = 40,
    seed: int | None = None,
) -> list[BackendResult]:
    logger.info("run_circuits")
    backend = get_backend(simulation_method, n_qubits)
    config = get_config(seed)
    return backend.run_circuits(circuits, n_shots, **config)


@worker.task()
def to_qasm3_str(circuit: Circuit) -> str:
    """Transforms a pytket circuit to a QASM3 string.

    Uses qiskits qasm3 module tket circuit -> qiskit circuit -> QASM3.

    :param circuit: The original pytket circuit.
    :type circuit: Circuit
    :return: The circuit in QASM3.
    :rtype: str
    """
    logger.info("run_circuit")
    return qasm3.dumps(tk_to_qiskit(circuit))


# Deprecated tasks


@worker.task()
def submit(circuits: list[Circuit], n_shots: int) -> list[BackendResult]:
    """Runs multiple circuits for n_shots on a simulated backend.

    :param circuits:  The circuits to simulate.
    :type circuits: list[Circuit]
    :param n_shots: Number of shots.
    :type n_shots: int
    :return: The aggregated results of the simulation for each circuit
    :rtype: list[BackendResult]
    """
    return AerBackend().run_circuits(circuits, n_shots=n_shots)


@worker.task()
def submit_single(circuit: Circuit, n_shots: int) -> BackendResult:
    """Runs a single circuit for n_shots on a simulated backend.

    :param circuit: The circuit to simulate.
    :type circuit: Circuit
    :param n_shots: Number of shots.
    :type n_shots: int
    :return: The aggregated results of the simulation.
    :rtype: BackendResult
    """
    return AerBackend().run_circuit(circuit, n_shots=n_shots)


def main():
    worker.app(argv)


if __name__ == "__main__":
    main()
