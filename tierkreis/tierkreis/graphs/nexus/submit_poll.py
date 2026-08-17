"""Sample graphs to interact with nexus using the Nexus Worker."""

# ruff: noqa: F821
from typing import NamedTuple

from tierkreis.builder import Graph
from tierkreis.builtins import tkr_sleep
from tierkreis.models import TKR, OpaqueType, Workflow
from tierkreis.nexus_worker import (
    get_results,
    is_running,
    start_execute_job,
    upload_circuit,
)

type Circuit = OpaqueType["pytket._tket.circuit.Circuit"]
type BackendResult = OpaqueType["pytket.backends.backendresult.BackendResult"]
type ExecuteJobRef = OpaqueType["qnexus.models.references.ExecuteJobRef"]


class UploadCircuitInputs(NamedTuple):
    """The inputs to upload a circuit.

    :fields:
        project_name (str): The name of the project to upload to.
        circuit (Circuit): The tket circuit to upload.
    """

    project_name: TKR[str]
    circuit: TKR[Circuit]


class JobInputs(NamedTuple):
    """The inputs to a nexus job.

    :fields:
        project_name (str): The name of the project to upload to.
        job_name (str): The name of the job.
        circuit (list[Circuit]): The list of circuits part of the job.
        n_shots (int): The number of shots (repetitions) of each circuit.
        backend_config (BackendConfig): The qnexus configuration of the backend.
    """

    project_name: TKR[str]
    job_name: TKR[str]
    circuits: TKR[list[Circuit]]
    n_shots: TKR[list[int]]
    backend_config: TKR[OpaqueType["qnexus.BackendConfig"]]


class _LoopOutputs(NamedTuple):
    results: TKR[list[BackendResult]]
    should_continue: TKR[bool]


def upload_circuit_graph() -> Workflow[UploadCircuitInputs, TKR[ExecuteJobRef]]:
    """Construct a graph to upload a circuit to nexus.

    :return: A uploading graph.
    :rtype: Graph[UploadCircuitInputs, TKR[ExecuteJobRef]]
    """
    g = Graph(UploadCircuitInputs, TKR[ExecuteJobRef])
    programme = g.task(upload_circuit(g.inputs.project_name, g.inputs.circuit))
    return g.finish_with_outputs(programme)  # type: ignore[arg-type]


def _polling_loop_body(
    polling_interval: float,
) -> Workflow[TKR[ExecuteJobRef], _LoopOutputs]:
    g = Graph(TKR[ExecuteJobRef], _LoopOutputs)
    pred = g.task(is_running(g.inputs))

    wait = g.ifelse(
        pred,
        g.task(tkr_sleep(g.const(polling_interval))),
        g.const(value=False),
    )
    results = g.ifelse(pred, g.const([]), g.task(get_results(g.inputs)))

    return g.finish_with_outputs(_LoopOutputs(results=results, should_continue=wait))


def nexus_submit_and_poll(
    polling_interval: float = 30.0,
) -> Workflow[JobInputs, TKR[list[BackendResult]]]:
    """Construct a graph submitting and polling a nexus job.

    :param polling_interval: The polling interval in seconds, defaults to 30.0
    :type polling_interval: float, optional
    :return: A graph performing submission and polling.
    :rtype: Graph[JobInputs, TKR[list[BackendResult]]]
    """
    g = Graph(JobInputs, TKR[list[BackendResult]])
    upload_inputs = g.map(
        lambda x: UploadCircuitInputs(g.inputs.project_name, x),
        g.inputs.circuits,
    )
    programmes = g.map(upload_circuit_graph(), upload_inputs)

    ref = g.task(
        start_execute_job(
            g.inputs.project_name,
            g.inputs.job_name,
            programmes,  # type: ignore[arg-type]
            g.inputs.n_shots,
            g.inputs.backend_config,  # type: ignore[arg-type]
        ),
    )

    res = g.loop(_polling_loop_body(polling_interval), ref)
    return g.finish_with_outputs(res.results)
