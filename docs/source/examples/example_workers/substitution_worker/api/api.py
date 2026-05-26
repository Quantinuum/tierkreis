"""Code generated from substitution_worker namespace. Please do not edit."""

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
        return "substitution_worker"
