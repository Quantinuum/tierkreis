from typing import Annotated, Any

from guppylang import guppy
from guppylang.emulator import EmulatorResult, EmulatorBuilder

from hugr.package import Package
from hugr.qsystem.result import QsysShot, DataValue
from pytket import Circuit
from pytket.backends.backendresult import BackendResult

from tierkreis import Worker
from tierkreis.controller.data.core import Deserializer, Serializer


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
