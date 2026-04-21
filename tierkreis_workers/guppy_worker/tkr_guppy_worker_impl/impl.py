from typing import Annotated, Any

from guppylang import comptime, guppy
from guppylang.emulator import EmulatorResult, EmulatorBuilder

from hugr.package import Package
from hugr.qsystem.result import QsysShot, DataValue
from pytket import Circuit
from pytket.backends.backendresult import BackendResult

from tierkreis import Worker
from tierkreis.controller.data.core import Deserializer, Serializer

from guppylang.std.builtins import array, result
from guppylang.std.quantum import cx, h, measure_array, qubit


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
    return guppy.load_pytket(name, circuit, use_arrays=use_arrays).compile()


@worker.task()
def to_backend_result(results: Result) -> BackendResult:
    return results.to_pytket()


@worker.task()
def ghz(size: int) -> Package:
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
