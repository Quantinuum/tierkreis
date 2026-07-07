"""Sample graphs to interact with nexus on the new stack using the Nexus Worker."""

# ruff: noqa: F821
from typing import NamedTuple

from tierkreis.builder import Graph
from tierkreis.builtins import tkr_sleep, at
from tierkreis.models import OpaqueType, TKR, Workflow
from tierkreis.nexus_worker import (
    get_results,
    is_running,
    start_single_job,
    upload_hugr,
)


type Package = OpaqueType["hugr.package.Package"]  # noqa: SLF001
type BackendResult = OpaqueType["pytket.backends.backendresult.BackendResult"]
type ExecuteJobRef = OpaqueType["qnexus.models.references.ExecuteJobRef"]


class UploadHugrInputs(NamedTuple):
    """The inputs to upload a circuit.

    :fields:
        project_name (str): The name of the project to upload to.
        hugr (Package): The the hugr to upload.
    """

    project_name: TKR[str]
    hugr: TKR[Package]


class JobInputs(NamedTuple):
    """The inputs to a nexus job.

    :fields:
        project_name (str): The name of the project to upload to.
        job_name (str): The name of the job.
        hugrs (list[Package]): The list of hugrs part of the job.
        n_shots (int): The number of shots (repetitions) of each circuit.
        backend_config (BackendConfig): The qnexus configuration of the backend.
    """

    project_name: TKR[str]
    job_name: TKR[str]
    hugrs: TKR[Package]
    n_shots: TKR[int]
    backend_config: TKR[OpaqueType["qnexus.BackendConfig"]]


class _LoopOutputs(NamedTuple):
    results: TKR[BackendResult]
    should_continue: TKR[bool]


def upload_hugr_graph() -> Workflow[UploadHugrInputs, TKR[ExecuteJobRef]]:
    """Construct a graph to upload a hugr to nexus.

    :return: A uploading graph.
    :rtype: Graph[UploadCircuitInputs, TKR[ExecuteJobRef]]
    """
    g = Graph(UploadHugrInputs, TKR[ExecuteJobRef])
    programme = g.task(upload_hugr(g.inputs.hugr, g.inputs.project_name))
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
    results = g.ifelse(pred, g.const([0]), g.task(get_results(g.inputs)))
    result = g.task(at(results, g.const(0)))  # type: ignore[arg-type] # type: ignore[arg-type]

    return g.finish_with_outputs(_LoopOutputs(results=result, should_continue=wait))


def nexus_submit_and_poll_hugr(
    polling_interval: float = 30.0,
) -> Workflow[JobInputs, TKR[BackendResult]]:
    """Construct a graph submitting and polling a nexus job.

    :param polling_interval: The polling interval in seconds, defaults to 30.0
    :type polling_interval: float, optional
    :return: A graph performing submission and polling.
    :rtype: Graph[JobInputs, TKR[BackendResult]]
    """
    g = Graph(JobInputs, TKR[BackendResult])
    program = g.eval(
        upload_hugr_graph(), UploadHugrInputs(g.inputs.project_name, g.inputs.hugrs)
    )

    ref = g.task(
        start_single_job(
            g.inputs.project_name,
            g.inputs.job_name,
            program,  # type: ignore[arg-type]
            g.inputs.n_shots,
            g.inputs.backend_config,  # type: ignore[arg-type]
        ),
    )

    res = g.loop(_polling_loop_body(polling_interval), ref)
    return g.finish_with_outputs(res.results)
