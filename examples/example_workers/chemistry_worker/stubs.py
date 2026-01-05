"""Code generated from chemistry_worker namespace. Please do not edit."""

from typing import NamedTuple, Protocol
from tierkreis.controller.data.models import TKR
from tierkreis.controller.data.types import Struct


class CompleteActiveSpace(Struct, Protocol):
    n: int  # noqa: F821 # fmt: skip
    n_ele: int  # noqa: F821 # fmt: skip


class Hamiltonian(Struct, Protocol):
    h0: float  # noqa: F821 # fmt: skip
    h1: list[list[float]]  # noqa: F821 # fmt: skip
    h2: list[list[list[list[float]]]]  # noqa: F821 # fmt: skip


class Molecule(Struct, Protocol):
    basis: str  # noqa: F821 # fmt: skip
    charge: int  # noqa: F821 # fmt: skip
    geometry: list[tuple[str, list[float]]]  # noqa: F821 # fmt: skip


class make_ham(NamedTuple):
    molecule: TKR[Molecule]  # noqa: F821 # fmt: skip
    mo_occ: TKR[list[int]]  # noqa: F821 # fmt: skip
    cas: TKR[CompleteActiveSpace]  # noqa: F821 # fmt: skip

    @staticmethod
    def out() -> type[TKR[Hamiltonian]]:  # noqa: F821 # fmt: skip
        return TKR[Hamiltonian]  # noqa: F821 # fmt: skip

    @property
    def namespace(self) -> str:
        return "chemistry_worker"
