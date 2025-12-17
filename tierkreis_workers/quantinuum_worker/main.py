from sys import argv
import time

import qnexus as qnx
from qnexus.models import IssuerEnum
from qnexus.models.references import ExecutionResultRef
from pytket._tket.circuit import Circuit
from pytket.backends.backendinfo import BackendInfo
from pytket.backends.backendresult import BackendResult
from pytket.extensions.quantinuum.backends.quantinuum import QuantinuumBackend
from pytket.passes import BasePass

from tierkreis import Worker
from tierkreis.exceptions import TierkreisError

from default_pass_quantinuum import default_compilation_pass

worker = Worker("quantinuum_worker")


@worker.task()
def get_backend_info(device_name: str) -> BackendInfo:
    """Retrieves a BackendInfo object for a given device name.

    :param device_name: The name of the device.
    :type device_name: str
    :raises TierkreisError: If the device is not found or not accessible.
    :return: The BackendInfo object for the device.
    :rtype: BackendInfo
    """
    all_devices = qnx.devices.get_all([IssuerEnum.QUANTINUUM])
    info = next(filter(lambda x: x.device_name == device_name, all_devices), None)
    if info is None:
        raise TierkreisError(
            f"Device {device_name} is not in the list of available Quantinuum devices"
        )
    return info.backend_info


@worker.task()
def compile_using_info(
    circuit: Circuit, backend_info: BackendInfo, optimisation_level: int = 2
) -> Circuit:
    base_pass = QuantinuumBackend.pass_from_info(
        backend_info, optimisation_level=optimisation_level
    )
    base_pass.apply(circuit)
    return circuit


@worker.task()
def backend_pass_from_info(
    backend_info: BackendInfo, optimisation_level: int = 2
) -> BasePass:
    """Returns a compilation pass according to the backend info.

    :param backend_info: Device information to use for compilation.
    :type backend_info: BackendInfo
    :param optimisation_level: The optimization level for the compilation, defaults to 2
    :type optimisation_level: int, optional
    :return: A compilation pass for the backend.
    :rtype: BasePass
    """
    return QuantinuumBackend.pass_from_info(
        backend_info, optimisation_level=optimisation_level
    )


@worker.task()
def fixed_pass() -> BasePass:
    """Returns a predefined compilation pass for Quantinuum devices.

    :return: The compilation pass.
    :rtype: BasePass
    """
    return default_compilation_pass()


@worker.task()
def compile_circuit_quantinuum(circuit: Circuit) -> Circuit:
    """Applies a predefined optimization pass for Quantinuum devices.

    The optimization pass corresponds to a level=3 optimization.

    :param circuit: The original circuit.
    :type circuit: Circuit
    :return: The optimized circuit.
    :rtype: Circuit
    """
    p = default_compilation_pass()
    p.apply(circuit)
    return circuit


@worker.task()
def compile_circuits_quantinuum(circuits: list[Circuit]) -> list[Circuit]:
    """Applies a predefined optimization pass for Quantinuum devices.

    :param circuits: A list of circuits to be optimized.
    :type circuits: list[Circuit]
    :return: The optimized circuits.
    :rtype: list[Circuit]
    """
    p = default_compilation_pass()
    for pytket_circuit in circuits:
        p.apply(pytket_circuit)
    return circuits


@worker.task()
def run_circuit(circuit: Circuit, n_shots: int, device_name: str) -> BackendResult:
    """Submits a circuit to a Quantinuum backend and returns the result.

    :param circuit: The circuit to be run.
    :type circuit: Circuit
    :param n_shots: The number of shots to run.
    :type n_shots: int
    :return: The backend result.
    :rtype: BackendResult
    """
    config = qnx.QuantinuumConfig(device_name=device_name)
    name = f"tkr-quantinuum-submission-{time.time()}"
    project = qnx.projects.get_or_create("tkr_submissions_project")
    qnx.context.set_active_project(project)
    circuit_ref = qnx.circuits.upload(circuit, project=project, name=name + "circuit")
    job_ref = qnx.start_execute_job(
        programs=circuit_ref,
        n_shots=[n_shots],
        backend_config=config,
        name=circuit.name or name,
    )
    qnx.jobs.wait_for(job_ref)
    ref_result = qnx.jobs.results(job_ref)[0]
    if not isinstance(ref_result, ExecutionResultRef):
        raise TierkreisError(f"Result incomplete: {ref_result}")
    backend_result = ref_result.download_result()
    if not isinstance(backend_result, BackendResult):
        raise TierkreisError(f"Result was not a backend result: {backend_result}")
    return backend_result


def main():
    worker.app(argv)


if __name__ == "__main__":
    main()
