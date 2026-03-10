"""Code generated from substitution_worker namespace. Please do not edit."""

from typing import Literal, NamedTuple
from tierkreis.controller.data.models import TKR

type Circuit = Literal["pytket._tket.circuit.Circuit"]


class substitute(NamedTuple):
    circuit: TKR[Circuit]  # noqa: F821 # fmt: skip
    a: TKR[float]  # noqa: F821 # fmt: skip
    b: TKR[float]  # noqa: F821 # fmt: skip
    c: TKR[float]  # noqa: F821 # fmt: skip

    @staticmethod
    def out() -> type[TKR[Circuit]]:  # noqa: F821 # fmt: skip
        return TKR[Circuit]  # noqa: F821 # fmt: skip

    @property
    def namespace(self) -> str:
        return "substitution_worker"
