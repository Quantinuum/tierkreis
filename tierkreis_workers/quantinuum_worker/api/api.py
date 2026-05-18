"""Code generated from quantinuum_worker namespace. Please do not edit."""

# ruff: noqa: F821
from typing import NamedTuple
from tierkreis.controller.data.models import TKR, OpaqueType


class get_backend_info(NamedTuple):
    device_name: TKR[str]

    @staticmethod
    def out() -> type[TKR[OpaqueType["pytket.backends.backendinfo.BackendInfo"]]]:
        return TKR[OpaqueType["pytket.backends.backendinfo.BackendInfo"]]

    @property
    def namespace(self) -> str:
        return "quantinuum_worker"


class compile_using_info(NamedTuple):
    circuit: TKR[OpaqueType["pytket._tket.circuit.Circuit"]]
    backend_info: TKR[OpaqueType["pytket.backends.backendinfo.BackendInfo"]]
    optimisation_level: TKR[int] | None = None

    @staticmethod
    def out() -> type[TKR[OpaqueType["pytket._tket.circuit.Circuit"]]]:
        return TKR[OpaqueType["pytket._tket.circuit.Circuit"]]

    @property
    def namespace(self) -> str:
        return "quantinuum_worker"


class backend_pass_from_info(NamedTuple):
    backend_info: TKR[OpaqueType["pytket.backends.backendinfo.BackendInfo"]]
    optimisation_level: TKR[int] | None = None

    @staticmethod
    def out() -> type[TKR[OpaqueType["pytket._tket.passes.BasePass"]]]:
        return TKR[OpaqueType["pytket._tket.passes.BasePass"]]

    @property
    def namespace(self) -> str:
        return "quantinuum_worker"


class fixed_pass(NamedTuple):
    @staticmethod
    def out() -> type[TKR[OpaqueType["pytket._tket.passes.BasePass"]]]:
        return TKR[OpaqueType["pytket._tket.passes.BasePass"]]

    @property
    def namespace(self) -> str:
        return "quantinuum_worker"


class compile_circuit_quantinuum(NamedTuple):
    circuit: TKR[OpaqueType["pytket._tket.circuit.Circuit"]]

    @staticmethod
    def out() -> type[TKR[OpaqueType["pytket._tket.circuit.Circuit"]]]:
        return TKR[OpaqueType["pytket._tket.circuit.Circuit"]]

    @property
    def namespace(self) -> str:
        return "quantinuum_worker"


class compile_circuits_quantinuum(NamedTuple):
    circuits: TKR[list[OpaqueType["pytket._tket.circuit.Circuit"]]]

    @staticmethod
    def out() -> type[TKR[list[OpaqueType["pytket._tket.circuit.Circuit"]]]]:
        return TKR[list[OpaqueType["pytket._tket.circuit.Circuit"]]]

    @property
    def namespace(self) -> str:
        return "quantinuum_worker"


class run_circuit(NamedTuple):
    circuit: TKR[OpaqueType["pytket._tket.circuit.Circuit"]]
    n_shots: TKR[int]
    device_name: TKR[str]

    @staticmethod
    def out() -> type[TKR[OpaqueType["pytket.backends.backendresult.BackendResult"]]]:
        return TKR[OpaqueType["pytket.backends.backendresult.BackendResult"]]

    @property
    def namespace(self) -> str:
        return "quantinuum_worker"
