"""Sample graphs to simulate quantum circuits on different backends."""

# ruff: noqa: F821
from typing import Literal, NamedTuple

from tierkreis.aer_worker import (
    get_compiled_circuit as aer_compile,
)
from tierkreis.aer_worker import (
    run_circuit as aer_run,
)
from tierkreis.builder import GraphBuilder
from tierkreis.builtins.stubs import str_eq, tkr_zip, untuple
from tierkreis.controller.data.models import TKR, OpaqueType
from tierkreis.qulacs_worker import (
    get_compiled_circuit as qulacs_compile,
)
from tierkreis.qulacs_worker import (
    run_circuit as qulacs_run,
)

type BackendResult = OpaqueType["pytket.backends.backendresult.BackendResult"]
type Circuit = OpaqueType["pytket._tket.circuit.Circuit"]  # noqa: SLF001


class SimulateJobInputs(NamedTuple):
    """Input to simulate multiple quantum circuits on a local backend.

    :fields:
        simulator_name (Literal): either 'aer' or 'qulacs'.
        circuits (list[Circuit]): The list of circuits to run.
        n_shots (int): The number of shots (repetitions) of each circuit.
        compilation_optimisation_level (int): tket optimisation level in [0,1,2,3].
    """

    simulator_name: TKR[Literal["aer", "qulacs"]]
    circuits: TKR[list[Circuit]]
    n_shots: TKR[list[int]]
    compilation_optimisation_level: TKR[int]


class SimulateJobInputsSingle(NamedTuple):
    """Input to simulate multiple quantum circuits on a local backend.

    :fields:
        simulator_name (Literal): either 'aer' or 'qulacs'.
        circuit_shots (tuple[Circuit, int]): The circuits to run and the number of shots.
        compilation_optimisation_level (int): tket optimisation level in [0,1,2,3].
    """

    simulator_name: TKR[Literal["aer", "qulacs"]]
    circuit_shots: TKR[tuple[Circuit, int]]
    compilation_optimisation_level: TKR[int]


def aer_simulate_single() -> GraphBuilder[SimulateJobInputsSingle, TKR[BackendResult]]:
    """Construct a graph to simulate a single circuit using qiskit aer.

    This ignores the simulator_name field.

    :return: The graph for the simulation.
    :rtype: GraphBuilder[SimulateJobInputsSingle, TKR[BackendResult]]
    """
    g = GraphBuilder(SimulateJobInputsSingle, TKR[BackendResult])
    circuit_shots = g.task(untuple(g.inputs.circuit_shots))

    compiled_circuit = g.task(
        aer_compile(
            circuit=circuit_shots.a,
            optimisation_level=g.inputs.compilation_optimisation_level,
        ),
    )
    res = g.task(aer_run(compiled_circuit, circuit_shots.b))
    g.outputs(res)
    return g


def qulacs_simulate_single() -> GraphBuilder[
    SimulateJobInputsSingle,
    TKR[BackendResult],
]:
    """Construct a graph to simulate a single circuit using qulacs.

    This ignores the simulator_name field.

    :return: The graph for the simulation.
    :rtype: GraphBuilder[SimulateJobInputsSingle, TKR[BackendResult]]
    """
    g = GraphBuilder(SimulateJobInputsSingle, TKR[BackendResult])
    circuit_shots = g.task(untuple(g.inputs.circuit_shots))

    compiled_circuit = g.task(
        qulacs_compile(
            circuit=circuit_shots.a,
            optimisation_level=g.inputs.compilation_optimisation_level,
        ),
    )
    res = g.task(qulacs_run(compiled_circuit, circuit_shots.b))
    g.outputs(res)
    return g


def compile_simulate_single() -> GraphBuilder[
    SimulateJobInputsSingle,
    TKR[BackendResult],
]:
    """CConstruct a graph to simulate a single job on either aer or qulacs.

    :return: The graph for the simulation.
    :rtype: GraphBuilder[ SimulateJobInputsSingle, TKR[BackendResult], ]
    """
    g = GraphBuilder(SimulateJobInputsSingle, TKR[BackendResult])

    aer_res = g.eval(aer_simulate_single(), g.inputs)
    qulacs_res = g.eval(qulacs_simulate_single(), g.inputs)
    res = g.ifelse(
        g.task(str_eq(g.inputs.simulator_name, g.const("aer"))),
        aer_res,
        qulacs_res,
    )

    g.outputs(res)
    return g


def compile_simulate() -> GraphBuilder[SimulateJobInputs, TKR[list[BackendResult]]]:
    """Construct a graph to simulate multiple jobs on either aer or qulacs.

    :return: The graph for the simulation.
    :rtype: GraphBuilder[SimulateJobInputs, TKR[list[BackendResult]]]
    """
    g = GraphBuilder(SimulateJobInputs, TKR[list[BackendResult]])

    circuits_shots = g.task(tkr_zip(g.inputs.circuits, g.inputs.n_shots))

    inputs = g.map(
        lambda x: SimulateJobInputsSingle(
            simulator_name=g.inputs.simulator_name,
            circuit_shots=x,
            compilation_optimisation_level=g.inputs.compilation_optimisation_level,
        ),
        circuits_shots,
    )
    res = g.map(compile_simulate_single(), inputs)

    g.outputs(res)
    return g
