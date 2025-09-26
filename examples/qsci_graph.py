# /// script
# requires-python = ">=3.12"
# dependencies = ["tierkreis", "pytket"]
#
# [tool.uv.sources]
# tierkreis = { path = "../tierkreis", editable = true }
# ///
import json
from pathlib import Path
from typing import NamedTuple
from uuid import UUID

from tierkreis.builder import GraphBuilder
from tierkreis.controller import run_graph
from tierkreis.controller.data.location import Loc
from tierkreis.controller.data.models import TKR, OpaqueType
from tierkreis.controller.executor.multiple import MultipleExecutor
from tierkreis.controller.executor.uv_executor import UvExecutor
from tierkreis.controller.storage.filestorage import ControllerFileStorage

from example_workers.chemistry_worker.stubs import (
    make_ham,
    Molecule,
    CompleteActiveSpace,
)
from example_workers.qsci_worker.stubs import (
    circuits_from_hamiltonians,
    energy_from_results,
    state_prep,
)
from tierkreis.aer_worker import submit_single
from tierkreis.quantinuum_worker import compile_circuit_quantinuum

root_loc = Loc()


def _compile_and_run() -> GraphBuilder[
    TKR[OpaqueType["pytket._tket.circuit.Circuit"]],  # noqa: F821
    TKR[OpaqueType["pytket.backends.backendresult.BackendResult"]],  # noqa: F821
]:
    g = GraphBuilder(
        TKR[OpaqueType["pytket._tket.circuit.Circuit"]],  # noqa: F821
        TKR[OpaqueType["pytket.backends.backendresult.BackendResult"]],  # noqa: F821
    )

    n_shots = g.const(500)
    compiled_circuit = g.task(compile_circuit_quantinuum(g.inputs))
    backend_result = g.task(submit_single(compiled_circuit, n_shots))

    g.outputs(backend_result)
    return g


class QSCIInputs(NamedTuple):
    molecule: TKR[Molecule]
    mo_occ: TKR[list[int]]  # TODO: check this type
    reference_state: TKR[list[int]]  # TODO: check this type
    cas_init: TKR[CompleteActiveSpace]
    cas_hsim: TKR[CompleteActiveSpace]
    t_step_list: TKR[list[float]]
    max_iteration_prep: TKR[int]
    max_cx_gates_hsim: TKR[int]
    atol: TKR[float]


class QSCIOutputs(NamedTuple):
    energy: TKR[float]


def qsci_graph() -> GraphBuilder[QSCIInputs, QSCIOutputs]:
    g = GraphBuilder(QSCIInputs, QSCIOutputs)
    # Separate tasks 'make_h_init'+'state_pre' and 'make_h_hsim' run in parallel
    ham_init = g.task(make_ham(g.inputs.molecule, g.inputs.mo_occ, g.inputs.cas_init))
    ham_hsim = g.task(make_ham(g.inputs.molecule, g.inputs.mo_occ, g.inputs.cas_hsim))

    adapt_circuit = g.task(
        state_prep(
            ham_init,
            g.inputs.reference_state,
            g.inputs.max_iteration_prep,
            g.inputs.atol,
            g.inputs.mo_occ,
            g.inputs.cas_init,
            g.inputs.cas_hsim,
        )
    )
    circuits = g.task(
        circuits_from_hamiltonians(
            ham_init,
            ham_hsim,
            adapt_circuit,
            g.inputs.t_step_list,
            g.inputs.cas_init,
            g.inputs.cas_hsim,
            g.inputs.mo_occ,
            g.inputs.max_cx_gates_hsim,
        )
    )
    backend_results = g.map(_compile_and_run(), circuits)
    energy = g.task(
        energy_from_results(
            ham_hsim,
            backend_results,
            g.inputs.mo_occ,
            g.inputs.cas_init,
            g.inputs.cas_hsim,
        )
    )

    g.outputs(QSCIOutputs(energy))
    return g


def main() -> None:
    """Configure our workflow execution and run it to completion."""
    # Assign a fixed uuid for our workflow.
    workflow_id = UUID(int=104)
    storage = ControllerFileStorage(workflow_id, name="qsci", do_cleanup=True)

    # Look for workers in the same directory as this file.
    registry_path = Path(__file__).parent
    # Look for workers in the `example_workers` directory.
    registry_path = Path(__file__).parent / "example_workers"
    custom_executor = UvExecutor(
        registry_path=registry_path, logs_path=storage.logs_path
    )
    common_registry_path = Path(__file__).parent.parent / "tierkreis_workers"
    common_executor = UvExecutor(
        registry_path=common_registry_path, logs_path=storage.logs_path
    )
    multi_executor = MultipleExecutor(
        common_executor,
        executors={"custom": custom_executor},
        assignments={"chemistry_worker": "custom", "qsci_worker": "custom"},
    )
    print("Starting workflow at location:", storage.logs_path)

    run_graph(
        storage,
        multi_executor,
        qsci_graph().get_data(),
        {
            k: json.dumps(v).encode()
            for k, v in (
                {
                    "molecule": {
                        "geometry": [
                            ["H", [0, 0, 0]],
                            ["H", [0, 0, 1.0]],
                            ["H", [0, 0, 2.0]],
                            ["H", [0, 0, 3.0]],
                        ],
                        "basis": "sto-3g",
                        "charge": 0,
                    },
                    "reference_state": [1, 1, 0, 0],
                    "cas_init": {
                        "n": 2,
                        "n_ele": 2,
                    },
                    "cas_hsim": {
                        "n": 4,
                        "n_ele": 4,
                    },
                    "max_cx_gates_hsim": 2000,
                    "mo_occ": [2, 2, 0, 0],
                    "t_step_list": [0.5, 1.0, 1.5],
                    "max_iteration_prep": 5,
                    "atol": 0.03,
                }
            ).items()
        },
        polling_interval_seconds=0.01,
    )
    output = json.loads(storage.read_output(root_loc, "energy"))
    print(output)


if __name__ == "__main__":
    main()
