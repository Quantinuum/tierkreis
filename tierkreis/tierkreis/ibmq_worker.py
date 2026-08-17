"""Code generated from ibmq_worker namespace. Please do not edit."""

# ruff: noqa: F821
from collections.abc import Sequence
from typing import NamedTuple

from tierkreis.controller.data.models import TKR, OpaqueType


class get_backend_info(NamedTuple):
    device_name: TKR[str]

    @staticmethod
    def out() -> type[TKR[OpaqueType["pytket.backends.backendinfo.BackendInfo"]]]:
        return TKR[OpaqueType["pytket.backends.backendinfo.BackendInfo"]]

    @property
    def namespace(self) -> str:
        return "ibmq_worker"


class backend_pass_from_info(NamedTuple):
    backend_info: TKR[OpaqueType["pytket.backends.backendinfo.BackendInfo"]]
    optimisation_level: TKR[int] | None = None

    @staticmethod
    def out() -> type[TKR[OpaqueType["pytket._tket.passes.BasePass"]]]:
        return TKR[OpaqueType["pytket._tket.passes.BasePass"]]

    @property
    def namespace(self) -> str:
        return "ibmq_worker"


class backend_default_compilation_pass(NamedTuple):
    device_name: TKR[str]
    optimisation_level: TKR[int] | None = None

    @staticmethod
    def out() -> type[TKR[OpaqueType["pytket._tket.passes.BasePass"]]]:
        return TKR[OpaqueType["pytket._tket.passes.BasePass"]]

    @property
    def namespace(self) -> str:
        return "ibmq_worker"


class fixed_pass(NamedTuple):
    coupling_map: TKR[Sequence[tuple[int, int]]]
    optimisation_level: TKR[int] | None = None

    @staticmethod
    def out() -> type[TKR[OpaqueType["pytket._tket.passes.BasePass"]]]:
        return TKR[OpaqueType["pytket._tket.passes.BasePass"]]

    @property
    def namespace(self) -> str:
        return "ibmq_worker"


class compile(NamedTuple):
    circuit: TKR[OpaqueType["pytket._tket.circuit.Circuit"]]
    device_name: TKR[str]
    optimisation_level: TKR[int] | None = None

    @staticmethod
    def out() -> type[TKR[OpaqueType["pytket._tket.circuit.Circuit"]]]:
        return TKR[OpaqueType["pytket._tket.circuit.Circuit"]]

    @property
    def namespace(self) -> str:
        return "ibmq_worker"


class compile_circuit_ibmq(NamedTuple):
    circuit: TKR[OpaqueType["pytket._tket.circuit.Circuit"]]
    device_name: TKR[str]
    optimisation_level: TKR[int] | None = None

    @staticmethod
    def out() -> type[TKR[OpaqueType["pytket._tket.circuit.Circuit"]]]:
        return TKR[OpaqueType["pytket._tket.circuit.Circuit"]]

    @property
    def namespace(self) -> str:
        return "ibmq_worker"


class compile_circuits_ibmq(NamedTuple):
    circuits: TKR[list[OpaqueType["pytket._tket.circuit.Circuit"]]]
    device_name: TKR[str]
    optimisation_level: TKR[int] | None = None

    @staticmethod
    def out() -> type[TKR[list[OpaqueType["pytket._tket.circuit.Circuit"]]]]:
        return TKR[list[OpaqueType["pytket._tket.circuit.Circuit"]]]

    @property
    def namespace(self) -> str:
        return "ibmq_worker"


class run_circuit(NamedTuple):
    circuit: TKR[OpaqueType["pytket._tket.circuit.Circuit"]]
    n_shots: TKR[int]
    device_name: TKR[str]

    @staticmethod
    def out() -> type[TKR[OpaqueType["pytket.backends.backendresult.BackendResult"]]]:
        return TKR[OpaqueType["pytket.backends.backendresult.BackendResult"]]

    @property
    def namespace(self) -> str:
        return "ibmq_worker"
