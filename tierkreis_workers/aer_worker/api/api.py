"""Code generated from aer_worker namespace. Please do not edit."""

# ruff: noqa: F821
from typing import NamedTuple
from types import NoneType
from tierkreis.controller.data.models import TKR, OpaqueType


class get_compiled_circuit(NamedTuple):
    circuit: TKR[OpaqueType["pytket._tket.circuit.Circuit"]]
    optimisation_level: TKR[int] | None = None
    timeout: TKR[int] | None = None
    simulation_method: TKR[str] | None = None
    n_qubits: TKR[int] | None = None

    @staticmethod
    def out() -> type[TKR[OpaqueType["pytket._tket.circuit.Circuit"]]]:
        return TKR[OpaqueType["pytket._tket.circuit.Circuit"]]

    @property
    def namespace(self) -> str:
        return "aer_worker"


class run_circuit(NamedTuple):
    circuit: TKR[OpaqueType["pytket._tket.circuit.Circuit"]]
    n_shots: TKR[int]
    simulation_method: TKR[str] | None = None
    n_qubits: TKR[int] | None = None
    seed: TKR[int | NoneType] | None = None

    @staticmethod
    def out() -> type[TKR[OpaqueType["pytket.backends.backendresult.BackendResult"]]]:
        return TKR[OpaqueType["pytket.backends.backendresult.BackendResult"]]

    @property
    def namespace(self) -> str:
        return "aer_worker"


class run_circuits(NamedTuple):
    circuits: TKR[list[OpaqueType["pytket._tket.circuit.Circuit"]]]
    n_shots: TKR[list[int]]
    simulation_method: TKR[str] | None = None
    n_qubits: TKR[int] | None = None
    seed: TKR[int | NoneType] | None = None

    @staticmethod
    def out() -> type[
        TKR[list[OpaqueType["pytket.backends.backendresult.BackendResult"]]]
    ]:
        return TKR[list[OpaqueType["pytket.backends.backendresult.BackendResult"]]]

    @property
    def namespace(self) -> str:
        return "aer_worker"


class to_qasm3_str(NamedTuple):
    circuit: TKR[OpaqueType["pytket._tket.circuit.Circuit"]]

    @staticmethod
    def out() -> type[TKR[str]]:
        return TKR[str]

    @property
    def namespace(self) -> str:
        return "aer_worker"


class submit(NamedTuple):
    circuits: TKR[list[OpaqueType["pytket._tket.circuit.Circuit"]]]
    n_shots: TKR[int]

    @staticmethod
    def out() -> type[
        TKR[list[OpaqueType["pytket.backends.backendresult.BackendResult"]]]
    ]:
        return TKR[list[OpaqueType["pytket.backends.backendresult.BackendResult"]]]

    @property
    def namespace(self) -> str:
        return "aer_worker"


class submit_single(NamedTuple):
    circuit: TKR[OpaqueType["pytket._tket.circuit.Circuit"]]
    n_shots: TKR[int]

    @staticmethod
    def out() -> type[TKR[OpaqueType["pytket.backends.backendresult.BackendResult"]]]:
        return TKR[OpaqueType["pytket.backends.backendresult.BackendResult"]]

    @property
    def namespace(self) -> str:
        return "aer_worker"
