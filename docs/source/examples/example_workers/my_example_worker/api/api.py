"""Code generated from my_example_worker namespace. Please do not edit."""

# ruff: noqa: F821
from typing import NamedTuple
from tierkreis.controller.data.models import TKR, OpaqueType


class substitute(NamedTuple):
    circuit: TKR[OpaqueType["pytket._tket.circuit.Circuit"]]
    a: TKR[float]
    b: TKR[float]
    c: TKR[float]

    @staticmethod
    def out() -> type[TKR[OpaqueType["pytket._tket.circuit.Circuit"]]]:
        return TKR[OpaqueType["pytket._tket.circuit.Circuit"]]

    @property
    def namespace(self) -> str:
        return "my_example_worker"


class symbolic_circuit(NamedTuple):
    @staticmethod
    def out() -> type[TKR[OpaqueType["pytket._tket.circuit.Circuit"]]]:
        return TKR[OpaqueType["pytket._tket.circuit.Circuit"]]

    @property
    def namespace(self) -> str:
        return "my_example_worker"


class ghz(NamedTuple):
    size: TKR[int]

    @staticmethod
    def out() -> type[TKR[OpaqueType["hugr.package.Package"]]]:
        return TKR[OpaqueType["hugr.package.Package"]]

    @property
    def namespace(self) -> str:
        return "my_example_worker"
