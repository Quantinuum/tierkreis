"""Code generated from guppy_worker namespace. Please do not edit."""

from typing import NamedTuple, Union
from types import NoneType
from tierkreis.controller.data.models import TKR, OpaqueType


class emulate(NamedTuple):
    hugr_package: TKR[OpaqueType["hugr.package.Package"]]  # noqa: F821 # fmt: skip
    n_qubits: TKR[int]  # noqa: F821 # fmt: skip
    n_shots: TKR[int]  # noqa: F821 # fmt: skip
    seed: TKR[Union[int, NoneType]] | None = None  # noqa: F821 # fmt: skip

    @staticmethod
    def out() -> type[TKR[OpaqueType["guppylang.emulator.result.EmulatorResult"]]]:  # noqa: F821 # fmt: skip
        return TKR[OpaqueType["guppylang.emulator.result.EmulatorResult"]]  # noqa: F821 # fmt: skip

    @property
    def namespace(self) -> str:
        return "guppy_worker"


class from_circuit(NamedTuple):
    circuit: TKR[OpaqueType["pytket._tket.circuit.Circuit"]]  # noqa: F821 # fmt: skip
    name: TKR[str]  # noqa: F821 # fmt: skip
    use_arrays: TKR[bool] | None = None  # noqa: F821 # fmt: skip

    @staticmethod
    def out() -> type[TKR[OpaqueType["hugr.package.Package"]]]:  # noqa: F821 # fmt: skip
        return TKR[OpaqueType["hugr.package.Package"]]  # noqa: F821 # fmt: skip

    @property
    def namespace(self) -> str:
        return "guppy_worker"


class to_backend_result(NamedTuple):
    results: TKR[OpaqueType["guppylang.emulator.result.EmulatorResult"]]  # noqa: F821 # fmt: skip

    @staticmethod
    def out() -> type[TKR[OpaqueType["pytket.backends.backendresult.BackendResult"]]]:  # noqa: F821 # fmt: skip
        return TKR[OpaqueType["pytket.backends.backendresult.BackendResult"]]  # noqa: F821 # fmt: skip

    @property
    def namespace(self) -> str:
        return "guppy_worker"


class ghz(NamedTuple):
    size: TKR[int]  # noqa: F821 # fmt: skip

    @staticmethod
    def out() -> type[TKR[OpaqueType["hugr.package.Package"]]]:  # noqa: F821 # fmt: skip
        return TKR[OpaqueType["hugr.package.Package"]]  # noqa: F821 # fmt: skip

    @property
    def namespace(self) -> str:
        return "guppy_worker"
