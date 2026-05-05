from typing import NamedTuple

import pytest
import qnexus as qnx
from guppy_worker import emulate, to_backend_result
from guppylang import guppy
from guppylang.std.builtins import array, result
from guppylang.std.quantum import cx, h, measure_array, qubit
from hugr.package import Package
from nexus_worker import cost, upload_hugr
from pytket_worker import backend_result_to_dict

from tierkreis.builder import Graph
from tierkreis.builtins.stubs import append, at, gt, tkr_range
from tierkreis.controller.data.models import TKR, OpaqueType
from tierkreis.controller.data.types import Workflow
from tierkreis.graphs.nexus.submit_poll_hugr import (
    JobInputs,
    nexus_submit_and_poll_hugr,
)
from tierkreis.storage import read_outputs
from tierkreis.cli.run_workflow import run_workflow


@guppy
def build_ghz_state() -> None:
    q = array(qubit() for _ in range(3))  # type: ignore
    h(q[0])
    for i in range(2):
        cx(q[i], q[i + 1])
    result("c", measure_array(q))


class MyInputs(NamedTuple):
    shots: TKR[int]
    n_qubits: TKR[int]
    package: TKR[Package]
    project_name: TKR[str]
    job_name: TKR[str]
    backend_config: TKR[OpaqueType["qnx.BackendConfig"]]


def guppy_nexus_graph() -> Workflow[MyInputs, TKR[dict[str, list[str]]]]:
    graph = Graph(MyInputs, TKR[dict[str, list[str]]])
    ref = graph.task(
        upload_hugr(
            graph.inputs.package,  # type: ignore
            graph.inputs.project_name,
            graph.inputs.job_name,
        )
    )
    run_cost = graph.task(cost(ref, graph.inputs.shots, graph.inputs.project_name))
    pred = graph.task(gt(run_cost, graph.const(0)))

    ## true case: cost > 0: simulate locally
    local = graph.task(
        emulate(graph.inputs.package, graph.inputs.n_qubits, graph.inputs.shots)  # type: ignore
    )
    if_true = graph.task(to_backend_result(local))

    ## false case: cost < 0: simulate on nexus

    empty_list = graph.task(tkr_range(graph.const(0), graph.const(0), graph.const(1)))
    shots = graph.task(append(empty_list, graph.inputs.shots))
    hugrs = graph.task(append(empty_list, graph.inputs.package))  # type: ignore
    simulate = graph.eval(
        nexus_submit_and_poll_hugr(),
        JobInputs(
            project_name=graph.inputs.project_name,
            job_name=graph.inputs.job_name,
            n_shots=shots,
            backend_config=graph.inputs.backend_config,  # type: ignore
            hugrs=hugrs,  # type: ignore
        ),
    )
    if_false = graph.task(at(simulate, graph.const(0)))

    out = graph.ifelse(pred, if_true, if_false)

    final = graph.task(backend_result_to_dict(out))
    return graph.finish_with_outputs(final)  # type: ignore


@pytest.mark.skip("Needs nexus credentials")
def test_guppy_nexus_integration() -> None:
    my_code = build_ghz_state.compile()
    wf = guppy_nexus_graph()
    config = qnx.models.HeliosConfig(
        system_name="Helios-1E",
        emulator_config=qnx.models.HeliosEmulatorConfig(
            n_qubits=3, simulator=qnx.models.StatevectorSimulator()
        ),
    )
    inputs = {
        "shots": 10,
        "n_qubits": 3,
        "package": my_code,
        "project_name": "tkr_test_project",
        "job_name": "tkr_test_hugr",
        "backend_config": config,
    }
    storage = run_workflow(wf, inputs, "Integration", run_id=2026)  # type: ignore
    out = read_outputs(wf, storage)
    assert isinstance(out, dict)
    assert "c" in out and isinstance(out["c"], list)
    assert len(out["c"]) == 10
