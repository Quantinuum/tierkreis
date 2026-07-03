import logging
from datetime import datetime

import qnexus as qnx
from qnexus.models.job_status import JobStatusEnum
from hugr.package import Package
from hugr.qsystem.result import QsysResult
from pytket._tket.circuit import Circuit
from pytket.backends.backendresult import BackendResult
from qnexus import BackendConfig
from qnexus.exceptions import ResourceFetchFailed
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
def start_single_job(
    project_name: str,
    job_name: str,
    circuit: ExecutionProgram,
    n_shots: int,
    backend_config: BackendConfig,
) -> ExecuteJobRef:
    """Wrapper around `qnx.start_execute_job`.

    :param project_name: The name of the nexus project to start the job in.
    :type project_name: str
    :param job_name: The name of the job to start.
    :type job_name: str
    :param circuit: The circuit to execute.
    :type circuit: ExecutionProgram
    :param n_shots: The number of shots for the circuit.
    :type n_shots: int
    :param backend_config: The backend configuration to use.
    :type backend_config: BackendConfig
    :return: A reference to the started execution job.
    :rtype: ExecuteJobRef
    """
    my_project_ref = qnx.projects.get_or_create(name=project_name)
    qnx.context.set_active_project(my_project_ref)
    return qnx.start_execute_job(circuit, n_shots, backend_config, job_name)


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

    if st in [JobStatusEnum.CANCELLING, JobStatusEnum.CANCELLED, JobStatusEnum.ERROR]:
        msg = f"Job status was {st}"
        raise TierkreisError(msg)

    return st != JobStatusEnum.COMPLETED


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

        if isinstance(result, QsysResult):
            result = result.to_pytket()
        assert isinstance(result, BackendResult)
        backend_results.append(result)
    return backend_results


@worker.task()
def upload_hugr(
    hugr_package: Package,
    project_name: str,
    name: str | None = None,
) -> HUGRRef:
    """Upload a Hugr to nexus

    :param hugr_package: The Hugr package to upload.
    :type hugr_package: Package
    :param project_name: The name of the nexus project to upload the Hugr to.
    :type project_name: str
    :param name: The name of the Hugr, defaults to a fallback name with timestamp.
    :type name: str | None, optional
    :return: The reference to the uploaded Hugr.
    :rtype: HUGRRef
    """
    if name is None:
        name = f"tkr HUGR Package from {datetime.now()}"
    my_project_ref = qnx.projects.get_or_create(name=project_name)
    qnx.context.set_active_project(my_project_ref)
    return qnx.hugr.upload(hugr_package, name=name)


@worker.task()
def cost(hugr_ref: HUGRRef | list[HUGRRef], n_shots: int, project_name: str) -> float:
    """Estimate the cost of running the Hugrs.

    :param hugr_ref: The reference(s) to the Hugr(s) to estimate the cost for.
    :type hugr_ref: HUGRRef | list[HUGRRef]
    :param n_shots: The number of shots to estimate the cost for.
    :type n_shots: int
    :param project_name: The name of the nexus project to estimate the cost in.
    :type project_name: str
    :return: The estimated cost.
    :rtype: float
    """
    my_project_ref = qnx.projects.get_or_create(name=project_name)
    qnx.context.set_active_project(my_project_ref)
    return qnx.hugr.cost(hugr_ref, n_shots)
