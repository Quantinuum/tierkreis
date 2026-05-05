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


class from_str(NamedTuple):
    package_str: TKR[str]  # noqa: F821 # fmt: skip

    @staticmethod
    def out() -> type[TKR[OpaqueType["hugr.package.Package"]]]:  # noqa: F821 # fmt: skip
        return TKR[OpaqueType["hugr.package.Package"]]  # noqa: F821 # fmt: skip

    @property
    def namespace(self) -> str:
        return "guppy_worker"


class to_str(NamedTuple):
    package: TKR[OpaqueType["hugr.package.Package"]]  # noqa: F821 # fmt: skip

    @staticmethod
    def out() -> type[TKR[str]]:  # noqa: F821 # fmt: skip
        return TKR[str]  # noqa: F821 # fmt: skip

    @property
    def namespace(self) -> str:
        return "guppy_worker"


class from_bytes(NamedTuple):
    package_bytes: TKR[bytes]  # noqa: F821 # fmt: skip

    @staticmethod
    def out() -> type[TKR[OpaqueType["hugr.package.Package"]]]:  # noqa: F821 # fmt: skip
        return TKR[OpaqueType["hugr.package.Package"]]  # noqa: F821 # fmt: skip

    @property
    def namespace(self) -> str:
        return "guppy_worker"


class to_bytes(NamedTuple):
    package: TKR[OpaqueType["hugr.package.Package"]]  # noqa: F821 # fmt: skip

    @staticmethod
    def out() -> type[TKR[bytes]]:  # noqa: F821 # fmt: skip
        return TKR[bytes]  # noqa: F821 # fmt: skip

    @property
    def namespace(self) -> str:
        return "guppy_worker"


class normalize(NamedTuple):
    package: TKR[OpaqueType["hugr.package.Package"]]  # noqa: F821 # fmt: skip

    @staticmethod
    def out() -> type[TKR[OpaqueType["hugr.package.Package"]]]:  # noqa: F821 # fmt: skip
        return TKR[OpaqueType["hugr.package.Package"]]  # noqa: F821 # fmt: skip

    @property
    def namespace(self) -> str:
        return "guppy_worker"


class apply_pytket_pass(NamedTuple):
    package: TKR[OpaqueType["hugr.package.Package"]]  # noqa: F821 # fmt: skip
    pytket_pass: TKR[OpaqueType["pytket._tket.passes.BasePass"]]  # noqa: F821 # fmt: skip

    @staticmethod
    def out() -> type[TKR[OpaqueType["hugr.package.Package"]]]:  # noqa: F821 # fmt: skip
        return TKR[OpaqueType["hugr.package.Package"]]  # noqa: F821 # fmt: skip

    @property
    def namespace(self) -> str:
        return "guppy_worker"


class remove_redundancies(NamedTuple):
    package: TKR[OpaqueType["hugr.package.Package"]]  # noqa: F821 # fmt: skip

    @staticmethod
    def out() -> type[TKR[OpaqueType["hugr.package.Package"]]]:  # noqa: F821 # fmt: skip
        return TKR[OpaqueType["hugr.package.Package"]]  # noqa: F821 # fmt: skip

    @property
    def namespace(self) -> str:
        return "guppy_worker"


class squash_rz_phased_x(NamedTuple):
    package: TKR[OpaqueType["hugr.package.Package"]]  # noqa: F821 # fmt: skip

    @staticmethod
    def out() -> type[TKR[OpaqueType["hugr.package.Package"]]]:  # noqa: F821 # fmt: skip
        return TKR[OpaqueType["hugr.package.Package"]]  # noqa: F821 # fmt: skip

    @property
    def namespace(self) -> str:
        return "guppy_worker"


class to_qir(NamedTuple):
    package: TKR[OpaqueType["hugr.package.Package"]]  # noqa: F821 # fmt: skip
    format: TKR[str]  # noqa: F821 # fmt: skip

    @staticmethod
    def out() -> type[TKR[Union[bytes, str]]]:  # noqa: F821 # fmt: skip
        return TKR[Union[bytes, str]]  # noqa: F821 # fmt: skip

    @property
    def namespace(self) -> str:
        return "guppy_worker"
