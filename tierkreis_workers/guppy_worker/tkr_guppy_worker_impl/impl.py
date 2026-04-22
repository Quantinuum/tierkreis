from typing import Annotated, Any

from guppylang import comptime, guppy
from guppylang.emulator import EmulatorBuilder, EmulatorResult
from guppylang.std.builtins import array, result
from guppylang.std.quantum import cx, h, measure_array, qubit
from hugr.package import Package
from hugr.qsystem.result import DataValue, QsysShot
from pytket import Circuit
from pytket.backends.backendresult import BackendResult
from tierkreis.controller.data.core import Deserializer, Serializer

from tierkreis import Worker

worker = Worker("guppy_worker")


def _serialize_emulator_result(result: EmulatorResult) -> list[dict[str, DataValue]]:
    return [shot.as_dict() for shot in result]


def _deserialize_emulator_result(result: Any) -> EmulatorResult:
    shots = [QsysShot([s]) for shot in result for s in shot.items()]
    return EmulatorResult(shots)


Result = Annotated[
    EmulatorResult,
    Serializer(_serialize_emulator_result, "json"),
    Deserializer(_deserialize_emulator_result, "json"),
]


@worker.task()
def emulate(
    hugr_package: Package,
    n_qubits: int,
    n_shots: int,
    seed: int | None = None,
) -> Result:
    """Emulate a hugr on a local emulator.

    :param hugr_package: The package to emulate.
    :type hugr_package: Package
    :param n_qubits: The number of qubits to emulate.
    :type n_qubits: int
    :param n_shots: The number of shots to take.
    :type n_shots: int
    :param seed: The seed for the random number generator, defaults to None
    :type seed: int | None, optional
    :return: The emulator result state.
    :rtype: Result
    """
    emulator = (
        EmulatorBuilder()
        .build(hugr_package, n_qubits)
        .with_shots(n_shots)
        .with_seed(seed)
    )
    results = emulator.run()
    return results


@worker.task()
def from_circuit(circuit: Circuit, name: str, use_arrays: bool = True) -> Package:
    """Build a hugr from a pytket circuit.

    :param circuit: The circuit to build the hugr from.
    :type circuit: Circuit
    :param name: The name of the hugr.
    :type name: str
    :param use_arrays: Use qubit arrays, defaults to True
    :type use_arrays: bool, optional
    :return: The compiled hugr.
    :rtype: Package
    """
    return guppy.load_pytket(name, circuit, use_arrays=use_arrays).compile()


@worker.task()
def to_backend_result(results: Result) -> BackendResult:
    """Transform a new stack result to a pytket result.

    :param results: The original result.
    :type results: Result
    :return: The corresponding pytket result.
    :rtype: BackendResult
    """
    return results.to_pytket()


@worker.task()
def ghz(size: int) -> Package:
    """Build a n-qubit GHZ quantum state as Hugr.

    :param size: The size of the state.
    :type size: int
    :return: A Hugr representing the GHZ state.
    :rtype: Package
    """
    n = guppy.nat_var("n")

    @guppy
    def build_ghz_state(q: array[qubit, n]) -> None:  # type: ignore
        h(q[0])
        for i in range(n - 1):  # type: ignore
            cx(q[i], q[i + 1])

    @guppy
    def main() -> None:
        q = array(qubit() for _ in range(comptime(size)))  # type: ignore
        build_ghz_state(q)

        result("c", measure_array(q))

    return main.compile()
