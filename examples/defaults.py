# ruff: noqa: F821
from pathlib import Path
from typing import NamedTuple
from uuid import UUID
from pytket.qasm.qasm import circuit_from_qasm
from tierkreis import run_graph
from tierkreis.consts import PACKAGE_PATH
from tierkreis.storage import FileStorage, read_outputs
from tierkreis.executor import UvExecutor
from tierkreis.builder import GraphBuilder
from tierkreis.controller.data.models import TKR, OpaqueType
from tierkreis.aer_worker import get_compiled_circuit

type Circuit = OpaqueType["pytket._tket.circuit.Circuit"]
type BackendResult = OpaqueType["pytket.backends.backendresult.BackendResult"]

circuit = circuit_from_qasm(Path(__file__).parent / "data" / "ghz_state_n23.qasm")


class InnerInputs(NamedTuple):
    circuit: TKR[Circuit]
    optimisation_level: TKR[int] | None = None


class OuterInputs(NamedTuple):
    circuit: TKR[Circuit]
    opt_level: TKR[int] | None = None


class OuterOutputs(NamedTuple):
    circuit_1: TKR[Circuit]
    circuit_2: TKR[Circuit]
    circuit_3: TKR[Circuit]
    extra_output: TKR[int] | None = None


def inner_graph() -> GraphBuilder[InnerInputs, TKR[Circuit]]:
    g = GraphBuilder(InnerInputs, TKR[Circuit])
    compiled_circuit = g.task(get_compiled_circuit(g.inputs.circuit))
    g.outputs(compiled_circuit)
    return g


def inner_graph_2() -> GraphBuilder[InnerInputs, TKR[Circuit]]:
    g = GraphBuilder(InnerInputs, TKR[Circuit])
    compiled_circuit = g.task(
        get_compiled_circuit(g.inputs.circuit, g.inputs.optimisation_level)
    )
    g.outputs(compiled_circuit)
    return g


def outer_graph() -> GraphBuilder[OuterInputs, OuterOutputs]:
    g = GraphBuilder(OuterInputs, OuterOutputs)
    compiled_circuit_1 = g.eval(inner_graph(), InnerInputs(g.inputs.circuit))
    compiled_circuit_2 = g.eval(inner_graph_2(), InnerInputs(g.inputs.circuit))
    compiled_circuit_3 = g.eval(
        inner_graph_2(), InnerInputs(g.inputs.circuit, g.const(2))
    )
    g.outputs(
        OuterOutputs(
            compiled_circuit_1,
            compiled_circuit_2,
            compiled_circuit_3,
            g.inputs.opt_level,
        )
    )
    return g


outer_g = outer_graph()

if __name__ == "__main__":
    storage = FileStorage(UUID(int=202), do_cleanup=True)
    executor = UvExecutor(PACKAGE_PATH / ".." / "tierkreis_workers", storage.logs_path)
    storage.clean_graph_files()
    run_graph(
        storage,
        executor,
        outer_g,
        {"circuit": circuit},
        polling_interval_seconds=0.1,
    )
    outputs = read_outputs(outer_g, storage)
    assert isinstance(outputs, dict)
    assert "circuit_1" in outputs
    assert "circuit_2" in outputs
    assert "circuit_3" in outputs
