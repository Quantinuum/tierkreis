"""Code generated from guppy_worker namespace. Please do not edit."""

# ruff: noqa: F821
from typing import NamedTuple, Union
from types import NoneType
from tierkreis.controller.data.models import TKR, OpaqueType


class emulate(NamedTuple):
    hugr_package: TKR[OpaqueType["hugr.package.Package"]]
    n_qubits: TKR[int]
    n_shots: TKR[int]
    seed: TKR[Union[int, NoneType]] | None = None

    @staticmethod
    def out() -> type[TKR[OpaqueType["guppylang.emulator.result.EmulatorResult"]]]:
        return TKR[OpaqueType["guppylang.emulator.result.EmulatorResult"]]

    @property
    def namespace(self) -> str:
        return "guppy_worker"


class from_circuit(NamedTuple):
    circuit: TKR[OpaqueType["pytket._tket.circuit.Circuit"]]
    name: TKR[str]
    use_arrays: TKR[bool] | None = None

    @staticmethod
    def out() -> type[TKR[OpaqueType["hugr.package.Package"]]]:
        return TKR[OpaqueType["hugr.package.Package"]]

    @property
    def namespace(self) -> str:
        return "guppy_worker"


class to_backend_result(NamedTuple):
    results: TKR[OpaqueType["guppylang.emulator.result.EmulatorResult"]]

    @staticmethod
    def out() -> type[TKR[OpaqueType["pytket.backends.backendresult.BackendResult"]]]:
        return TKR[OpaqueType["pytket.backends.backendresult.BackendResult"]]

    @property
    def namespace(self) -> str:
        return "guppy_worker"


class ghz(NamedTuple):
    size: TKR[int]

    @staticmethod
    def out() -> type[TKR[OpaqueType["hugr.package.Package"]]]:
        return TKR[OpaqueType["hugr.package.Package"]]

    @property
    def namespace(self) -> str:
        return "guppy_worker"


class from_str(NamedTuple):
    package_str: TKR[str]

    @staticmethod
    def out() -> type[TKR[OpaqueType["hugr.package.Package"]]]:
        return TKR[OpaqueType["hugr.package.Package"]]

    @property
    def namespace(self) -> str:
        return "guppy_worker"


class to_str(NamedTuple):
    package: TKR[OpaqueType["hugr.package.Package"]]

    @staticmethod
    def out() -> type[TKR[str]]:
        return TKR[str]

    @property
    def namespace(self) -> str:
        return "guppy_worker"


class from_bytes(NamedTuple):
    package_bytes: TKR[bytes]

    @staticmethod
    def out() -> type[TKR[OpaqueType["hugr.package.Package"]]]:
        return TKR[OpaqueType["hugr.package.Package"]]

    @property
    def namespace(self) -> str:
        return "guppy_worker"


class to_bytes(NamedTuple):
    package: TKR[OpaqueType["hugr.package.Package"]]

    @staticmethod
    def out() -> type[TKR[bytes]]:
        return TKR[bytes]

    @property
    def namespace(self) -> str:
        return "guppy_worker"


class normalize(NamedTuple):
    package: TKR[OpaqueType["hugr.package.Package"]]

    @staticmethod
    def out() -> type[TKR[OpaqueType["hugr.package.Package"]]]:
        return TKR[OpaqueType["hugr.package.Package"]]

    @property
    def namespace(self) -> str:
        return "guppy_worker"


class apply_pytket_pass(NamedTuple):
    package: TKR[OpaqueType["hugr.package.Package"]]
    pytket_pass: TKR[OpaqueType["pytket._tket.passes.BasePass"]]

    @staticmethod
    def out() -> type[TKR[OpaqueType["hugr.package.Package"]]]:
        return TKR[OpaqueType["hugr.package.Package"]]

    @property
    def namespace(self) -> str:
        return "guppy_worker"


class remove_redundancies(NamedTuple):
    package: TKR[OpaqueType["hugr.package.Package"]]

    @staticmethod
    def out() -> type[TKR[OpaqueType["hugr.package.Package"]]]:
        return TKR[OpaqueType["hugr.package.Package"]]

    @property
    def namespace(self) -> str:
        return "guppy_worker"


class squash_rz_phased_x(NamedTuple):
    package: TKR[OpaqueType["hugr.package.Package"]]

    @staticmethod
    def out() -> type[TKR[OpaqueType["hugr.package.Package"]]]:
        return TKR[OpaqueType["hugr.package.Package"]]

    @property
    def namespace(self) -> str:
        return "guppy_worker"


class to_qir(NamedTuple):
    package: TKR[OpaqueType["hugr.package.Package"]]
    format: TKR[str]

    @staticmethod
    def out() -> type[TKR[Union[bytes, str]]]:
        return TKR[Union[bytes, str]]

    @property
    def namespace(self) -> str:
        return "guppy_worker"
