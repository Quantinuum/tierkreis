from typing import Annotated, Any

from guppylang.decorator import guppy
from guppylang.emulator import EmulatorBuilder, EmulatorResult
from guppylang.std.builtins import array, result, comptime
from guppylang.std.quantum import cx, h, measure_array, qubit
from hugr.package import Package
from hugr.qsystem.result import DataValue, QsysShot
from pytket import Circuit
from pytket.backends.backendresult import BackendResult
from pytket.passes import BasePass, RemoveRedundancies, SquashRzPhasedX
from tierkreis.controller.data.core import Deserializer, Serializer
from tierkreis.exceptions import TierkreisError
from tket.passes import NormalizeGuppy, PytketHugrPass

from tierkreis import Worker

worker = Worker("guppy_worker")


def _serialize_emulator_result(result: EmulatorResult) -> list[dict[str, DataValue]]:
    return [shot.as_dict() for shot in result]


def _deserialize_emulator_result(result: Any) -> EmulatorResult:
    if isinstance(result, list):
        shots = [QsysShot(s) for shot in result for s in shot]
        return EmulatorResult(shots)
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


@worker.task()
def from_str(package_str: str) -> Package:
    """Build a Hugr from a string representation of a quantum program.

    :param package_str: The string representation of the quantum program.
    :type package_str: str
    :return: The compiled Hugr.
    :rtype: Package
    """
    return Package.from_str(package_str)


@worker.task()
def to_str(package: Package) -> str:
    """Convert a Hugr to a string representation of a quantum program.

    :param package: The Hugr package to convert.
    :type package: Package
    :return: The string representation of the Hugr.
    :rtype: str
    """
    return package.to_str()


@worker.task()
def from_bytes(package_bytes: bytes) -> Package:
    """Build a Hugr from a bytes representation of a quantum program.

    :param package_bytes: The bytes representation of the quantum program.
    :type package_bytes: bytes
    :return: The compiled Hugr.
    :rtype: Package
    """
    return Package.from_bytes(package_bytes)


@worker.task()
def to_bytes(package: Package) -> bytes:
    """Convert a Hugr to a bytes representation of a quantum program.

    :param package: The Hugr package to convert.
    :type package: Package
    :return: The bytes representation of the Hugr.
    :rtype: bytes
    """
    return package.to_bytes()


@worker.task()
def normalize(package: Package) -> Package:
    """Normalize a Hugr package.

    :param package: The Hugr package to normalize.
    :type package: Package
    :return: The normalized Hugr package.
    :rtype: Package
    """
    normalize_pass = NormalizeGuppy()  # type: ignore
    return normalize_pass(package.modules[0]).to_package()


@worker.task()
def apply_pytket_pass(package: Package, pytket_pass: BasePass) -> Package:
    """Apply a pytket pass to a Hugr package.

    :param package: The Hugr package to apply the pass to.
    :type package: Package
    :param pytket_pass: The pytket pass to apply.
    :type pytket_pass: BasePass
    :return: The Hugr package after applying the pass.
    :rtype: Package
    """
    pass_ = PytketHugrPass(pytket_pass)  # type: ignore
    result = pass_(package.modules[0])
    return result.to_package()


@worker.task()
def remove_redundancies(package: Package) -> Package:
    """Remove redundant operations from a Hugr package.

    :param package: The Hugr package to optimize.
    :type package: Package
    :return: The optimized Hugr package.
    :rtype: Package
    """
    pass_ = PytketHugrPass(RemoveRedundancies())  # type: ignore
    result = pass_(package.modules[0])
    return result.to_package()


@worker.task()
def squash_rz_phased_x(package: Package) -> Package:
    """Squash Rz and PhasedX gates in a Hugr package.

    :param package: The Hugr package to optimize.
    :type package: Package
    :return: The optimized Hugr package.
    :rtype: Package
    """
    pass_ = PytketHugrPass(SquashRzPhasedX())  # type: ignore
    result = pass_(package.modules[0])
    return result.to_package()


@worker.task()
def to_qir(package: Package) -> bytes | str:
    """Convert a Hugr package to a QIR representation.

    :param package: The Hugr package to convert.
    :type package: Package
    :return: The QIR representation of the Hugr package.
    :rtype: bytes | str
    """
    try:
        from hugr_qir.hugr_to_qir import hugr_to_qir
    except ImportError | ModuleNotFoundError:
        msg = "Could not resolve hugr_qir.hugr_to_qir. Please ensure that the hugr_qir package is installed."
        raise TierkreisError(msg)
    qir_bytes = hugr_to_qir(package.to_bytes())
    return qir_bytes
