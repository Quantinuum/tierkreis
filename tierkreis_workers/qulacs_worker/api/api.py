"""Code generated from qulacs_worker namespace. Please do not edit."""

# ruff: noqa: F821
from typing import NamedTuple, Union
from types import NoneType
from tierkreis.controller.data.models import TKR, OpaqueType


class get_compiled_circuit(NamedTuple):
    circuit: TKR[OpaqueType["pytket._tket.circuit.Circuit"]]
    optimisation_level: TKR[int] | None = None
    result_type: TKR[str] | None = None
    gpu_sim: TKR[bool] | None = None

    @staticmethod
    def out() -> type[TKR[OpaqueType["pytket._tket.circuit.Circuit"]]]:
        return TKR[OpaqueType["pytket._tket.circuit.Circuit"]]

    @property
    def namespace(self) -> str:
        return "qulacs_worker"


class run_circuit(NamedTuple):
    circuit: TKR[OpaqueType["pytket._tket.circuit.Circuit"]]
    n_shots: TKR[int]
    result_type: TKR[str] | None = None
    gpu_sim: TKR[bool] | None = None
    seed: TKR[Union[int, NoneType]] | None = None

    @staticmethod
    def out() -> type[TKR[OpaqueType["pytket.backends.backendresult.BackendResult"]]]:
        return TKR[OpaqueType["pytket.backends.backendresult.BackendResult"]]

    @property
    def namespace(self) -> str:
        return "qulacs_worker"


class run_circuits(NamedTuple):
    circuits: TKR[list[OpaqueType["pytket._tket.circuit.Circuit"]]]
    n_shots: TKR[list[int]]
    result_type: TKR[str] | None = None
    gpu_sim: TKR[bool] | None = None
    seed: TKR[Union[int, NoneType]] | None = None

    @staticmethod
    def out() -> type[
        TKR[list[OpaqueType["pytket.backends.backendresult.BackendResult"]]]
    ]:
        return TKR[list[OpaqueType["pytket.backends.backendresult.BackendResult"]]]

    @property
    def namespace(self) -> str:
        return "qulacs_worker"
