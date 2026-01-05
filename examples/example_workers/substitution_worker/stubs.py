"""Code generated from substitution_worker namespace. Please do not edit."""

from typing import Literal, NamedTuple, Sequence, TypeVar, Generic, Protocol, Union
from types import NoneType
from tierkreis.controller.data.models import TKR, OpaqueType
from tierkreis.controller.data.types import PType, Struct



class substitute(NamedTuple):
    circuit: TKR[OpaqueType["pytket._tket.circuit.Circuit"]]  # noqa: F821 # fmt: skip
    a: TKR[float]  # noqa: F821 # fmt: skip
    b: TKR[float]  # noqa: F821 # fmt: skip
    c: TKR[float]  # noqa: F821 # fmt: skip

    @staticmethod
    def out() -> type[TKR[OpaqueType["pytket._tket.circuit.Circuit"]]]: # noqa: F821 # fmt: skip
        return TKR[OpaqueType["pytket._tket.circuit.Circuit"]] # noqa: F821 # fmt: skip

    @property
    def namespace(self) -> str:
        return "substitution_worker" 
