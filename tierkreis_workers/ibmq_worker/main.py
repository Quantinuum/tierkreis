from collections.abc import Sequence
from sys import argv

from default_pass import IBMQ_GATE_SET, default_compilation_pass
from pytket._tket.circuit import Circuit
from pytket.architecture import Architecture
from pytket.backends.backendinfo import BackendInfo
from pytket.backends.backendresult import BackendResult
from pytket.extensions.qiskit.backends.ibm import IBMQBackend
from pytket.passes import BasePass

from tierkreis import Worker
from tierkreis.exceptions import TierkreisError

worker = Worker("ibmq_worker")


@worker.task()
def get_backend_info(device_name: str) -> BackendInfo:
    """Retrieves a BackendInfo object for a given device name.

    :param device_name: The name of the device.
    :type device_name: str
    :raises TierkreisError: If the device is not found or not accessible.
    :return: The BackendInfo object for the device.
    :rtype: BackendInfo
    """
    info = next(
        filter(
            lambda x: x.device_name == device_name,
            IBMQBackend.available_devices(),
        ),
        None,
    )
    if info is None:
        msg = f"Device {device_name} is not in the list of available IBMQ devices"
        raise TierkreisError(
            msg,
        )
    return info


@worker.task()
def backend_pass_from_info(
    backend_info: BackendInfo,
    optimisation_level: int = 2,
) -> BasePass:
    """Returns a compilation pass according to the backend info.

    :param backend_info: Device information to use for compilation.
    :type backend_info: BackendInfo
    :param optimisation_level: The optimization level for the compilation, defaults to 2
    :type optimisation_level: int, optional
    :return: A compilation pass for the backend.
    :rtype: BasePass
    """
    return IBMQBackend.pass_from_info(
        backend_info,
        optimisation_level=optimisation_level,
    )


@worker.task()
def backend_default_compilation_pass(
    device_name: str,
    optimisation_level: int = 2,
) -> BasePass:
    """Returns the default compilation pass for a given device name.

    :param device_name: The name of the device.
    :type device_name: str
    :param optimisation_level: The optimization level for the compilation, defaults to 2
    :type optimisation_level: int, optional
    :return: The default compilation pass for the backend.
    :rtype: BasePass
    """
    backend = IBMQBackend(device_name)
    return backend.default_compilation_pass(optimisation_level)


@worker.task()
def fixed_pass(
    coupling_map: Sequence[tuple[int, int]],
    optimisation_level: int = 2,
) -> BasePass:
    """Returns a predefined compilation pass for IBMQ devices.

    :param coupling_map: The coupling map for the device.
    :type coupling_map: Sequence[tuple[int, int]]
    :param optimisation_level: The optimization level for the compilation, defaults to 2
    :type optimisation_level: int, optional
    :return: A compilation pass for the backend.
    :rtype: BasePass
    """
    arch = Architecture(coupling_map)
    return default_compilation_pass(optimisation_level, arch, IBMQ_GATE_SET)


@worker.task()
def compile(
    circuit: Circuit,
    device_name: str,
    optimisation_level: int = 2,
) -> Circuit:
    """Gets a compiled circuit for a given device name.

    :param circuit: The original circuit to compile.
    :type circuit: Circuit
    :param device_name: The name of the device to compile for.
    :type device_name: str
    :param optimisation_level: The optimization level for the compilation, defaults to 2
    :type optimisation_level: int, optional
    :return: The compiled circuit.
    :rtype: Circuit
    """
    backend = IBMQBackend(device_name)
    compilation_pass = backend.default_compilation_pass(optimisation_level)
    compilation_pass.apply(circuit)
    return circuit


@worker.task()
def compile_circuit_ibmq(
    circuit: Circuit,
    device_name: str,
    optimisation_level: int = 2,
) -> Circuit:
    """Applies a predefined optimization pass for IBMQ devices.

    The optimization pass corresponds to a level=3 optimization.

    :param circuit: The original circuit.
    :type circuit: Circuit
    :return: The optimized circuit.
    :rtype: Circuit
    """
    p = IBMQBackend(device_name).default_compilation_pass(optimisation_level)
    p.apply(circuit)
    return circuit


@worker.task()
def compile_circuits_ibmq(
    circuits: list[Circuit],
    device_name: str,
    optimisation_level: int = 2,
) -> list[Circuit]:
    """Applies a predefined optimization pass for IBMQ devices.

    :param circuits: A list of circuits to be optimized.
    :type circuits: list[Circuit]
    :return: The optimized circuits.
    :rtype: list[Circuit]
    """
    p = IBMQBackend(device_name).default_compilation_pass(optimisation_level)
    for pytket_circuit in circuits:
        p.apply(pytket_circuit)
    return circuits


@worker.task()
def run_circuit(circuit: Circuit, n_shots: int, device_name: str) -> BackendResult:
    """Submits a circuit to an IBMQ backend and returns the result.

    :param circuit: The circuit to be run.
    :type circuit: Circuit
    :param n_shots: The number of shots to run.
    :type n_shots: int
    :return: The backend result.
    :rtype: BackendResult
    """
    backend = IBMQBackend(device_name)
    return backend.run_circuit(circuit, n_shots)


def main() -> None:
    worker.app(argv)


if __name__ == "__main__":
    main()
