"""Code generated from quantinuum_worker namespace. Please do not edit."""

from typing import NamedTuple
from tierkreis.controller.data.models import TKR, OpaqueType


class get_backend_info(NamedTuple):
    device_name: TKR[str]  # noqa: F821 # fmt: skip

    @staticmethod
    def out() -> type[TKR[OpaqueType["pytket.backends.backendinfo.BackendInfo"]]]:  # noqa: F821 # fmt: skip
        return TKR[OpaqueType["pytket.backends.backendinfo.BackendInfo"]]  # noqa: F821 # fmt: skip

    @property
    def namespace(self) -> str:
        return "quantinuum_worker"


class compile_using_info(NamedTuple):
    circuit: TKR[OpaqueType["pytket._tket.circuit.Circuit"]]  # noqa: F821 # fmt: skip
    backend_info: TKR[OpaqueType["pytket.backends.backendinfo.BackendInfo"]]  # noqa: F821 # fmt: skip
    optimisation_level: TKR[int] | None = None  # noqa: F821 # fmt: skip

    @staticmethod
    def out() -> type[TKR[OpaqueType["pytket._tket.circuit.Circuit"]]]:  # noqa: F821 # fmt: skip
        return TKR[OpaqueType["pytket._tket.circuit.Circuit"]]  # noqa: F821 # fmt: skip

    @property
    def namespace(self) -> str:
        return "quantinuum_worker"


class backend_pass_from_info(NamedTuple):
    backend_info: TKR[OpaqueType["pytket.backends.backendinfo.BackendInfo"]]  # noqa: F821 # fmt: skip
    optimisation_level: TKR[int] | None = None  # noqa: F821 # fmt: skip

    @staticmethod
    def out() -> type[TKR[OpaqueType["pytket._tket.passes.BasePass"]]]:  # noqa: F821 # fmt: skip
        return TKR[OpaqueType["pytket._tket.passes.BasePass"]]  # noqa: F821 # fmt: skip

    @property
    def namespace(self) -> str:
        return "quantinuum_worker"


class fixed_pass(NamedTuple):
    @staticmethod
    def out() -> type[TKR[OpaqueType["pytket._tket.passes.BasePass"]]]:  # noqa: F821 # fmt: skip
        return TKR[OpaqueType["pytket._tket.passes.BasePass"]]  # noqa: F821 # fmt: skip

    @property
    def namespace(self) -> str:
        return "quantinuum_worker"


class compile_circuit_quantinuum(NamedTuple):
    circuit: TKR[OpaqueType["pytket._tket.circuit.Circuit"]]  # noqa: F821 # fmt: skip

    @staticmethod
    def out() -> type[TKR[OpaqueType["pytket._tket.circuit.Circuit"]]]:  # noqa: F821 # fmt: skip
        return TKR[OpaqueType["pytket._tket.circuit.Circuit"]]  # noqa: F821 # fmt: skip

    @property
    def namespace(self) -> str:
        return "quantinuum_worker"


class compile_circuits_quantinuum(NamedTuple):
    circuits: TKR[list[OpaqueType["pytket._tket.circuit.Circuit"]]]  # noqa: F821 # fmt: skip

    @staticmethod
    def out() -> type[TKR[list[OpaqueType["pytket._tket.circuit.Circuit"]]]]:  # noqa: F821 # fmt: skip
        return TKR[list[OpaqueType["pytket._tket.circuit.Circuit"]]]  # noqa: F821 # fmt: skip

    @property
    def namespace(self) -> str:
        return "quantinuum_worker"


class run_circuit(NamedTuple):
    circuit: TKR[OpaqueType["pytket._tket.circuit.Circuit"]]  # noqa: F821 # fmt: skip
    n_shots: TKR[int]  # noqa: F821 # fmt: skip
    device_name: TKR[str]  # noqa: F821 # fmt: skip

    @staticmethod
    def out() -> type[TKR[OpaqueType["pytket.backends.backendresult.BackendResult"]]]:  # noqa: F821 # fmt: skip
        return TKR[OpaqueType["pytket.backends.backendresult.BackendResult"]]  # noqa: F821 # fmt: skip

    @property
    def namespace(self) -> str:
        return "quantinuum_worker"
