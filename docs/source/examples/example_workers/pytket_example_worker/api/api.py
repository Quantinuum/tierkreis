"""Code generated from substitution_worker namespace. Please do not edit."""

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
        return "substitution_worker"
