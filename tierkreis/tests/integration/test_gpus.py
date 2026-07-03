from pathlib import Path
from typing import NamedTuple
from uuid import UUID


import qnexus as qnx
from gpu_worker import get_devices
from guppylang.decorator import guppy
from guppylang.std.builtins import array, result
from guppylang.std.quantum import cx, h, measure_array, qubit
from hugr.package import Package
from pytket_worker import backend_result_to_dict


from tierkreis.builder import Graph
from tierkreis.builtins.stubs import append, at
from tierkreis.controller import run_graph
from tierkreis.controller.data.models import TKR, OpaqueType
from tierkreis.controller.data.types import Workflow
from tierkreis.controller.executor.hpc.pbs import PBSExecutor
from tierkreis.controller.executor.multiple import MultipleExecutor
from tierkreis.controller.executor.uv_executor import UvExecutor
from tierkreis.controller.storage.filestorage import ControllerFileStorage
from tierkreis.controller.storage.protocol import ControllerStorage
from tierkreis.graphs.nexus.submit_poll_hugr import (
    JobInputs,
    nexus_submit_and_poll_hugr,
)
from tierkreis.storage import read_outputs


from tierkreis.controller.executor.hpc.job_spec import (
    JobSpec,
    ResourceSpec,
)


# def slurm_spec() -> JobSpec:
#     return JobSpec(
#         job_name="test_job",
#         account="...",
#         command=("/root/.local/bin/uv run ..."),
#         resource=ResourceSpec(
#             nodes=1,
#         ),
#         walltime="00:15:00",
#     )


def pbs(storage: ControllerStorage) -> PBSExecutor:
    spec = JobSpec(
        job_name="test_job",
        account="seitzp",
        command="uv run",
        resource=ResourceSpec(nodes=1, memory_gb=None, gpus_per_node=1),
        walltime="00:15:00",
        queue="normal",
        extra_scheduler_args={"-P": "59001004"},
        modules=[
            "cuda/12.8.1",
        ],
    )
    executor = PBSExecutor(
        spec=spec,
        registry_path=Path.home() / "Projects" / "tierkreis",
        logs_path=storage.logs_path,
        command="qsub",
    )
    return executor


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


class MyOutputs(NamedTuple):
    results: TKR[dict[str, list[str]]]
    devices: TKR[str]


def guppy_nexus_graph() -> Workflow[MyInputs, MyOutputs]:
    graph = Graph(MyInputs, MyOutputs)
    gpus = graph.task(get_devices())
    empty_list = graph.const([])
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
    out = graph.task(at(simulate, graph.const(0)))
    final = graph.task(backend_result_to_dict(out))
    return graph.finish_with_outputs(MyOutputs(results=final, devices=gpus))


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
    storage = ControllerFileStorage(
        UUID(int=1), name="test_integration", do_cleanup=True
    )
    pbs_executor = pbs(storage)
    test_workers_path = Path(__file__).parent.parent / "workers"
    default = UvExecutor(test_workers_path, storage.logs_path)
    executor = MultipleExecutor(default, {"pbs": pbs_executor}, {"gpu_worker": "pbs"})
    storage.clean_graph_files()
    run_graph(storage, executor, wf, inputs)
    out = read_outputs(wf, storage)
    print(out)
    assert out is not None


if __name__ == "__main__":
    test_guppy_nexus_integration()
