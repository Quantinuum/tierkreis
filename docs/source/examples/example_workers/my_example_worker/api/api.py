"""Code generated from my_example_worker namespace. Please do not edit."""

from typing import NamedTuple
from tierkreis.controller.data.models import TKR, OpaqueType


class substitute(NamedTuple):
    circuit: TKR[OpaqueType["pytket._tket.circuit.Circuit"]]  # noqa: F821 # fmt: skip
    a: TKR[float]  # noqa: F821 # fmt: skip
    b: TKR[float]  # noqa: F821 # fmt: skip
    c: TKR[float]  # noqa: F821 # fmt: skip

    @staticmethod
    def out() -> type[TKR[OpaqueType["pytket._tket.circuit.Circuit"]]]:  # noqa: F821 # fmt: skip
        return TKR[OpaqueType["pytket._tket.circuit.Circuit"]]  # noqa: F821 # fmt: skip

    @property
    def namespace(self) -> str:
        return "my_example_worker"


class symbolic_circuit(NamedTuple):
    @staticmethod
    def out() -> type[TKR[OpaqueType["pytket._tket.circuit.Circuit"]]]:  # noqa: F821 # fmt: skip
        return TKR[OpaqueType["pytket._tket.circuit.Circuit"]]  # noqa: F821 # fmt: skip

    @property
    def namespace(self) -> str:
        return "my_example_worker"


class ghz(NamedTuple):
    size: TKR[int]  # noqa: F821 # fmt: skip

    @staticmethod
    def out() -> type[TKR[OpaqueType["hugr.package.Package"]]]:  # noqa: F821 # fmt: skip
        return TKR[OpaqueType["hugr.package.Package"]]  # noqa: F821 # fmt: skip

    @property
    def namespace(self) -> str:
        return "my_example_worker"
