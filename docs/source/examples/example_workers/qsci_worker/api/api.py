"""Code generated from qsci_worker namespace. Please do not edit."""

# ruff: noqa: F821
from typing import NamedTuple, Protocol
from tierkreis.controller.data.models import TKR, OpaqueType
from tierkreis.controller.data.types import Struct


class CompleteActiveSpace(Struct, Protocol):
    n: int
    n_ele: int


class Hamiltonian(Struct, Protocol):
    h0: float
    h1: list[list[float]]
    h2: list[list[list[list[float]]]]


class Molecule(Struct, Protocol):
    basis: str
    charge: int
    geometry: list[tuple[str, list[float]]]


class state_prep(NamedTuple):
    ham_init: TKR[Hamiltonian]
    reference_state: TKR[list[int]]
    max_iteration_prep: TKR[int]
    atol: TKR[float]
    mo_occ: TKR[list[int]]
    cas_init: TKR[CompleteActiveSpace]
    cas_hsim: TKR[CompleteActiveSpace]

    @staticmethod
    def out() -> type[TKR[OpaqueType["pytket._tket.circuit.Circuit"]]]:
        return TKR[OpaqueType["pytket._tket.circuit.Circuit"]]

    @property
    def namespace(self) -> str:
        return "qsci_worker"


class circuits_from_hamiltonians(NamedTuple):
    ham_init: TKR[Hamiltonian]
    ham_hsim: TKR[Hamiltonian]
    adapt_circuit: TKR[OpaqueType["pytket._tket.circuit.Circuit"]]
    t_step_list: TKR[list[float]]
    cas_init: TKR[CompleteActiveSpace]
    cas_hsim: TKR[CompleteActiveSpace]
    mo_occ: TKR[list[int]]
    max_cx_gates: TKR[int]

    @staticmethod
    def out() -> type[TKR[list[OpaqueType["pytket._tket.circuit.Circuit"]]]]:
        return TKR[list[OpaqueType["pytket._tket.circuit.Circuit"]]]

    @property
    def namespace(self) -> str:
        return "qsci_worker"


class energy_from_results(NamedTuple):
    ham_hsim: TKR[Hamiltonian]
    backend_results: TKR[
        list[OpaqueType["pytket.backends.backendresult.BackendResult"]]
    ]
    mo_occ: TKR[list[int]]
    cas_init: TKR[CompleteActiveSpace]
    cas_hsim: TKR[CompleteActiveSpace]

    @staticmethod
    def out() -> type[TKR[float]]:
        return TKR[float]

    @property
    def namespace(self) -> str:
        return "qsci_worker"


class make_ham(NamedTuple):
    molecule: TKR[Molecule]
    mo_occ: TKR[list[int]]
    cas: TKR[CompleteActiveSpace]

    @staticmethod
    def out() -> type[TKR[Hamiltonian]]:
        return TKR[Hamiltonian]

    @property
    def namespace(self) -> str:
        return "qsci_worker"
