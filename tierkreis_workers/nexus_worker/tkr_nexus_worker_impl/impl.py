import logging
import warnings
from datetime import datetime
from time import sleep

import qnexus as qnx
from hugr.package import Package
from pytket._tket.circuit import Circuit
from pytket.backends.backendresult import BackendResult
from pytket.backends.status import StatusEnum
from qnexus import BackendConfig
from qnexus.exceptions import ResourceFetchFailed
from qnexus.models import QuantinuumConfig
from qnexus.models.references import (
    ExecuteJobRef,
    ExecutionProgram,
    ExecutionResultRef,
    HUGRRef,
)

from tierkreis import Worker
from tierkreis.exceptions import TierkreisError

logger = logging.getLogger(__name__)
worker = Worker("nexus_worker")


@worker.task()
def upload_circuit(project_name: str, circ: Circuit) -> ExecutionProgram:
    """Wrapper around `qnx.circuits.upload`.

    :param project_name: The name of the nexus project to upload the circuit to.
    :type project_name: str
    :param circ: The circuit to upload.
    :type circ: Circuit
    :return: A reference to the uploaded circuit.
    :rtype: ExecutionProgram
    """
    my_project_ref = qnx.projects.get_or_create(name=project_name)
    circuit_name = circ.name or f"circuit_{datetime.now()}"
    qnx.context.set_active_project(my_project_ref)
    return qnx.circuits.upload(name=circuit_name, circuit=circ, project=my_project_ref)


@worker.task()
def start_execute_job(
    project_name: str,
    job_name: str,
    circuits: list[ExecutionProgram],
    n_shots: list[int],
    backend_config: BackendConfig,
) -> ExecuteJobRef:
    """Wrapper around `qnx.start_execute_job`.

    :param project_name: The name of the nexus project to start the job in.
    :type project_name: str
    :param job_name: The name of the job to start.
    :type job_name: str
    :param circuits: The circuits to execute.
    :type circuits: list[ExecutionProgram]
    :param n_shots: The number of shots for each circuit.
    :type n_shots: list[int]
    :param backend_config: The backend configuration to use.
    :type backend_config: BackendConfig
    :return: A reference to the started execution job.
    :rtype: ExecuteJobRef
    """
    my_project_ref = qnx.projects.get_or_create(name=project_name)
    qnx.context.set_active_project(my_project_ref)
    return qnx.start_execute_job(circuits, n_shots, backend_config, job_name)


@worker.task()
def is_running(execute_ref: ExecuteJobRef) -> bool:
    """Wrapper around `qnx.jobs.status`.

    :param execute_ref: The reference to the execution job.
    :type execute_ref: ExecuteJobRef
    :raises TierkreisError: If the job was cancelled or errored.
    :return: True if the job is still running, False otherwise.
    :rtype: bool
    """
    try:
        st = qnx.jobs.status(execute_ref).status
    except ResourceFetchFailed:
        return True

    if st in [StatusEnum.CANCELLING, StatusEnum.CANCELLED, StatusEnum.ERROR]:
        msg = f"Job status was {st}"
        raise TierkreisError(msg)

    return st != StatusEnum.COMPLETED


@worker.task()
def get_results(execute_ref: ExecuteJobRef) -> list[BackendResult]:
    """Wrapper around `qnx.results` and `qnx.download_result`.

    :param execute_ref: The reference to the execution job.
    :type execute_ref: ExecuteJobRef
    :return: A list of backend results for each circuit in the job.
    :rtype: list[BackendResult]
    """
    execute_job_result_refs = qnx.jobs.results(execute_ref)
    backend_results: list[BackendResult] = []
    for i in range(len(execute_job_result_refs)):
        ref_result = execute_job_result_refs[i]
        if not isinstance(ref_result, ExecutionResultRef):
            msg = f"Result incomplete: {ref_result}"
            raise TierkreisError(msg)
        result = ref_result.download_result()
        assert isinstance(result, BackendResult)
        backend_results.append(result)
    return backend_results


@worker.task()
def upload_hugr(
    hugr_package: Package,
    project_name: str,
    name: str | None = None,
) -> HUGRRef:
    if name is None:
        name = f"tkr HUGR Package from {datetime.now()}"
    my_project_ref = qnx.projects.get_or_create(name=project_name)
    qnx.context.set_active_project(my_project_ref)
    return qnx.hugr.upload(hugr_package, name=name)


@worker.task()
def cost(hugr_ref: HUGRRef | list[HUGRRef], n_shots: int) -> float:
    return qnx.hugr.cost(hugr_ref, n_shots)


def start_execute_hugr(
    project_name: str,
    job_name: str,
    hugrs: list[HUGRRef],
    n_shots: list[int],
    backend_config: BackendConfig,
) -> ExecuteJobRef:
    my_project_ref = qnx.projects.get_or_create(name=project_name)
    qnx.context.set_active_project(my_project_ref)
    return qnx.start_execute_job(hugrs, n_shots, backend_config, job_name)  # type: ignore list is not covariant here


## DEPRECATED TASKS ##


@worker.task()
def check_status(execute_ref: ExecuteJobRef) -> str:
    warnings.warn("check_status is deprecated, use is_running instead", stacklevel=2)
    sleep(30)
    try:
        return str(qnx.jobs.status(execute_ref).status)
    except ResourceFetchFailed:
        return str(StatusEnum.SUBMITTED)


@worker.task()
def submit(
    circuits: list[Circuit], n_shots: int, project_name: str | None = None
) -> ExecuteJobRef:
    warnings.warn(
        "submit is deprecated, use upload_circuit and start_execute_job instead",
        stacklevel=2,
    )
    if project_name is None:
        project_name = "Riken-Test"
    my_project_ref = qnx.projects.get_or_create(name=project_name)
    qnx.context.set_active_project(my_project_ref)

    my_circuit_refs: list[ExecutionProgram] = []
    for circ in circuits:
        my_circuit_refs.append(
            qnx.circuits.upload(
                name=f"My Circuit from {datetime.now()}",
                circuit=circ,
                project=my_project_ref,
            ),
        )

    return qnx.start_execute_job(
        programs=my_circuit_refs,
        name=f"My Execute Job from {datetime.now()}",
        n_shots=[n_shots] * len(my_circuit_refs),
        backend_config=QuantinuumConfig(device_name="reimei-E"),
        project=my_project_ref,
    )
