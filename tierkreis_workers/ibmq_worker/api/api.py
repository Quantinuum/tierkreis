"""Code generated from ibmq_worker namespace. Please do not edit."""

from typing import NamedTuple, Sequence
from tierkreis.controller.data.models import TKR, OpaqueType


class get_backend_info(NamedTuple):
    device_name: TKR[str]  # noqa: F821 # fmt: skip

    @staticmethod
    def out() -> type[TKR[OpaqueType["pytket.backends.backendinfo.BackendInfo"]]]:  # noqa: F821 # fmt: skip
        return TKR[OpaqueType["pytket.backends.backendinfo.BackendInfo"]]  # noqa: F821 # fmt: skip

    @property
    def namespace(self) -> str:
        return "ibmq_worker"


class backend_pass_from_info(NamedTuple):
    backend_info: TKR[OpaqueType["pytket.backends.backendinfo.BackendInfo"]]  # noqa: F821 # fmt: skip
    optimisation_level: TKR[int] | None = None  # noqa: F821 # fmt: skip

    @staticmethod
    def out() -> type[TKR[OpaqueType["pytket._tket.passes.BasePass"]]]:  # noqa: F821 # fmt: skip
        return TKR[OpaqueType["pytket._tket.passes.BasePass"]]  # noqa: F821 # fmt: skip

    @property
    def namespace(self) -> str:
        return "ibmq_worker"


class backend_default_compilation_pass(NamedTuple):
    device_name: TKR[str]  # noqa: F821 # fmt: skip
    optimisation_level: TKR[int] | None = None  # noqa: F821 # fmt: skip

    @staticmethod
    def out() -> type[TKR[OpaqueType["pytket._tket.passes.BasePass"]]]:  # noqa: F821 # fmt: skip
        return TKR[OpaqueType["pytket._tket.passes.BasePass"]]  # noqa: F821 # fmt: skip

    @property
    def namespace(self) -> str:
        return "ibmq_worker"


class fixed_pass(NamedTuple):
    coupling_map: TKR[Sequence[tuple[int, int]]]  # noqa: F821 # fmt: skip
    optimisation_level: TKR[int] | None = None  # noqa: F821 # fmt: skip

    @staticmethod
    def out() -> type[TKR[OpaqueType["pytket._tket.passes.BasePass"]]]:  # noqa: F821 # fmt: skip
        return TKR[OpaqueType["pytket._tket.passes.BasePass"]]  # noqa: F821 # fmt: skip

    @property
    def namespace(self) -> str:
        return "ibmq_worker"


class compile(NamedTuple):
    circuit: TKR[OpaqueType["pytket._tket.circuit.Circuit"]]  # noqa: F821 # fmt: skip
    device_name: TKR[str]  # noqa: F821 # fmt: skip
    optimisation_level: TKR[int] | None = None  # noqa: F821 # fmt: skip

    @staticmethod
    def out() -> type[TKR[OpaqueType["pytket._tket.circuit.Circuit"]]]:  # noqa: F821 # fmt: skip
        return TKR[OpaqueType["pytket._tket.circuit.Circuit"]]  # noqa: F821 # fmt: skip

    @property
    def namespace(self) -> str:
        return "ibmq_worker"


class compile_circuit_ibmq(NamedTuple):
    circuit: TKR[OpaqueType["pytket._tket.circuit.Circuit"]]  # noqa: F821 # fmt: skip
    device_name: TKR[str]  # noqa: F821 # fmt: skip
    optimisation_level: TKR[int] | None = None  # noqa: F821 # fmt: skip

    @staticmethod
    def out() -> type[TKR[OpaqueType["pytket._tket.circuit.Circuit"]]]:  # noqa: F821 # fmt: skip
        return TKR[OpaqueType["pytket._tket.circuit.Circuit"]]  # noqa: F821 # fmt: skip

    @property
    def namespace(self) -> str:
        return "ibmq_worker"


class compile_circuits_ibmq(NamedTuple):
    circuits: TKR[list[OpaqueType["pytket._tket.circuit.Circuit"]]]  # noqa: F821 # fmt: skip
    device_name: TKR[str]  # noqa: F821 # fmt: skip
    optimisation_level: TKR[int] | None = None  # noqa: F821 # fmt: skip

    @staticmethod
    def out() -> type[TKR[list[OpaqueType["pytket._tket.circuit.Circuit"]]]]:  # noqa: F821 # fmt: skip
        return TKR[list[OpaqueType["pytket._tket.circuit.Circuit"]]]  # noqa: F821 # fmt: skip

    @property
    def namespace(self) -> str:
        return "ibmq_worker"


class run_circuit(NamedTuple):
    circuit: TKR[OpaqueType["pytket._tket.circuit.Circuit"]]  # noqa: F821 # fmt: skip
    n_shots: TKR[int]  # noqa: F821 # fmt: skip
    device_name: TKR[str]  # noqa: F821 # fmt: skip

    @staticmethod
    def out() -> type[TKR[OpaqueType["pytket.backends.backendresult.BackendResult"]]]:  # noqa: F821 # fmt: skip
        return TKR[OpaqueType["pytket.backends.backendresult.BackendResult"]]  # noqa: F821 # fmt: skip

    @property
    def namespace(self) -> str:
        return "ibmq_worker"
