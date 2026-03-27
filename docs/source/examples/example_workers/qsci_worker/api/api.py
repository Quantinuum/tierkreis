"""Code generated from qsci_worker namespace. Please do not edit."""

from typing import Literal, NamedTuple, Protocol
from tierkreis.controller.data.models import TKR
from tierkreis.controller.data.types import Struct

type Circuit = Literal["pytket._tket.circuit.Circuit"]
type BackendResult = Literal["pytket.backends.backendresult.BackendResult"]


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


class state_prep(NamedTuple):
    ham_init: TKR[Hamiltonian]  # noqa: F821 # fmt: skip
    reference_state: TKR[list[int]]  # noqa: F821 # fmt: skip
    max_iteration_prep: TKR[int]  # noqa: F821 # fmt: skip
    atol: TKR[float]  # noqa: F821 # fmt: skip
    mo_occ: TKR[list[int]]  # noqa: F821 # fmt: skip
    cas_init: TKR[CompleteActiveSpace]  # noqa: F821 # fmt: skip
    cas_hsim: TKR[CompleteActiveSpace]  # noqa: F821 # fmt: skip

    @staticmethod
    def out() -> type[TKR[Circuit]]:  # noqa: F821 # fmt: skip
        return TKR[Circuit]  # noqa: F821 # fmt: skip

    @property
    def namespace(self) -> str:
        return "qsci_worker"


class circuits_from_hamiltonians(NamedTuple):
    ham_init: TKR[Hamiltonian]  # noqa: F821 # fmt: skip
    ham_hsim: TKR[Hamiltonian]  # noqa: F821 # fmt: skip
    adapt_circuit: TKR[Circuit]  # noqa: F821 # fmt: skip
    t_step_list: TKR[list[float]]  # noqa: F821 # fmt: skip
    cas_init: TKR[CompleteActiveSpace]  # noqa: F821 # fmt: skip
    cas_hsim: TKR[CompleteActiveSpace]  # noqa: F821 # fmt: skip
    mo_occ: TKR[list[int]]  # noqa: F821 # fmt: skip
    max_cx_gates: TKR[int]  # noqa: F821 # fmt: skip

    @staticmethod
    def out() -> type[TKR[list[Circuit]]]:  # noqa: F821 # fmt: skip
        return TKR[list[Circuit]]  # noqa: F821 # fmt: skip

    @property
    def namespace(self) -> str:
        return "qsci_worker"


class energy_from_results(NamedTuple):
    ham_hsim: TKR[Hamiltonian]  # noqa: F821 # fmt: skip
    backend_results: TKR[list[BackendResult]]  # noqa: F821 # fmt: skip
    mo_occ: TKR[list[int]]  # noqa: F821 # fmt: skip
    cas_init: TKR[CompleteActiveSpace]  # noqa: F821 # fmt: skip
    cas_hsim: TKR[CompleteActiveSpace]  # noqa: F821 # fmt: skip

    @staticmethod
    def out() -> type[TKR[float]]:  # noqa: F821 # fmt: skip
        return TKR[float]  # noqa: F821 # fmt: skip

    @property
    def namespace(self) -> str:
        return "qsci_worker"


class make_ham(NamedTuple):
    molecule: TKR[Molecule]  # noqa: F821 # fmt: skip
    mo_occ: TKR[list[int]]  # noqa: F821 # fmt: skip
    cas: TKR[CompleteActiveSpace]  # noqa: F821 # fmt: skip

    @staticmethod
    def out() -> type[TKR[Hamiltonian]]:  # noqa: F821 # fmt: skip
        return TKR[Hamiltonian]  # noqa: F821 # fmt: skip

    @property
    def namespace(self) -> str:
        return "qsci_worker"
