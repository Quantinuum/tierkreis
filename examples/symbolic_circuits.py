# /// script
# requires-python = ">=3.12"
# dependencies = [
#  "pytket",
#  "tierkreis",
# ]
#
# [tool.uv.sources]
# tierkreis = { path = "../tierkreis", editable = true }
# ///
import json
from pathlib import Path
from sys import argv
from typing import NamedTuple
from uuid import UUID

from pytket._tket.circuit import Circuit, fresh_symbol
from tierkreis.builder import GraphBuilder
from tierkreis.controller import run_graph
from tierkreis.controller.data.location import Loc
from tierkreis.controller.data.models import TKR, OpaqueType
from tierkreis.controller.executor.hpc.job_spec import JobSpec, ResourceSpec
from tierkreis.controller.executor.hpc.pjsub import PJSUBExecutor
from tierkreis.controller.storage.filestorage import ControllerFileStorage
from tierkreis.controller.executor.multiple import MultipleExecutor
from tierkreis.controller.executor.uv_executor import UvExecutor

from example_workers.substitution_worker.stubs import substitute
from tierkreis.pytket_worker import (
    add_measure_all,
    optimise_phase_gadgets,
    expectation,
)
from tierkreis.aer_worker import submit_single

root_loc = Loc()


def build_ansatz() -> Circuit:
    # TODO: This isn't a very good ansatz
    a = fresh_symbol("a")
    b = fresh_symbol("b")
    c = fresh_symbol("c")
    circ = Circuit(4)
    circ.CX(0, 1)
    circ.CX(1, 2)
    circ.CX(2, 3)
    circ.Rz(a, 3)
    circ.CX(2, 3)
    circ.CX(1, 2)
    circ.CX(0, 1)
    circ.Rz(b, 0)
    circ.CX(0, 1)
    circ.CX(1, 2)
    circ.CX(2, 3)
    circ.Rz(c, 3)
    circ.CX(2, 3)
    circ.CX(1, 2)
    circ.CX(0, 1)
    return circ


class SymbolicCircuitsInputs(NamedTuple):
    a: TKR[float]
    b: TKR[float]
    c: TKR[float]
    ansatz: TKR[OpaqueType["pytket._tket.circuit.Circuit"]]  # noqa: F821


class SymbolicCircuitsOutputs(NamedTuple):
    expectation: TKR[float]


def symbolic_execution() -> GraphBuilder:
    """A graph that substitutes 3 parameters into a circuit and gets an expectation value."""
    g = GraphBuilder(SymbolicCircuitsInputs, SymbolicCircuitsOutputs)
    a = g.inputs.a
    b = g.inputs.b
    c = g.inputs.c
    ansatz = g.inputs.ansatz
    n_shots = g.const(100)

    substituted_circuit = g.task(substitute(a=a, b=b, c=c, circuit=ansatz))
    measurement_circuit = g.task(add_measure_all(circuit=substituted_circuit))

    # TODO: A better compilation pass
    compiled_circuit = g.task(optimise_phase_gadgets(circuit=measurement_circuit))
    backend_result = g.task(submit_single(circuit=compiled_circuit, n_shots=n_shots))
    av = g.task(expectation(backend_result=backend_result))

    g.outputs(SymbolicCircuitsOutputs(expectation=av))
    return g


def pjsub_uv_executor(
    group_name: str, registry_path: Path, logs_path: Path
) -> PJSUBExecutor:
    spec = JobSpec(
        job_name="tkr_symbolic_ciruits",
        account=group_name,
        command="env UV_PROJECT_ENVIRONMENT=compute_venv uv run main.py",
        resource=ResourceSpec(nodes=1, memory_gb=None, gpus_per_node=None),
        walltime="00:15:00",
        output_path=Path(logs_path),
        error_path=Path(logs_path),
        include_no_check_directory_flag=True,
    )
    return PJSUBExecutor(spec=spec, registry_path=registry_path, logs_path=logs_path)


def main(pjsub_group_name: str | None) -> None:
    """Configure our workflow execution and run it to completion."""
    ansatz = build_ansatz()

    # Assign a fixed uuid for our workflow.
    workflow_id = UUID(int=101)
    storage = ControllerFileStorage(
        workflow_id,
        name="symbolic_circuits",
        do_cleanup=True,
    )

    # Look for workers in the `example_workers` directory.
    registry_path = Path(__file__).parent / "example_workers"
    custom_executor = (
        pjsub_uv_executor(pjsub_group_name, registry_path, storage.logs_path)
        if pjsub_group_name is not None
        else UvExecutor(registry_path=registry_path, logs_path=storage.logs_path)
    )
    common_registry_path = Path(__file__).parent.parent / "tierkreis_workers"
    common_executor = UvExecutor(
        registry_path=common_registry_path, logs_path=storage.logs_path
    )
    multi_executor = MultipleExecutor(
        common_executor,
        executors={"custom": custom_executor},
        assignments={"substitution_worker": "custom"},
    )

    print("Starting workflow at location:", storage.logs_path)
    run_graph(
        storage,
        multi_executor,
        symbolic_execution().data,
        {
            "ansatz": ansatz,
            "a": 0.2,
            "b": 0.55,
            "c": 0.75,
        },
        polling_interval_seconds=0.1,
    )
    output = json.loads(storage.read_output(root_loc, "expectation"))
    print(output)


if __name__ == "__main__":
    pjsub_group_name = argv[1] if len(argv) > 1 else None
    main(pjsub_group_name)
