"""Code generated from ibmq_worker namespace. Please do not edit."""

from typing import Literal, NamedTuple, Sequence
from tierkreis.controller.data.models import TKR

type BackendInfo = Literal["pytket.backends.backendinfo.BackendInfo"]
type BasePass = Literal["pytket._tket.passes.BasePass"]
type Circuit = Literal["pytket._tket.circuit.Circuit"]
type BackendResult = Literal["pytket.backends.backendresult.BackendResult"]


class get_backend_info(NamedTuple):
    device_name: TKR[str]  # noqa: F821 # fmt: skip

    @staticmethod
    def out() -> type[TKR[BackendInfo]]:  # noqa: F821 # fmt: skip
        return TKR[BackendInfo]  # noqa: F821 # fmt: skip

    @property
    def namespace(self) -> str:
        return "ibmq_worker"


class backend_pass_from_info(NamedTuple):
    backend_info: TKR[BackendInfo]  # noqa: F821 # fmt: skip
    optimisation_level: TKR[int] | None = None  # noqa: F821 # fmt: skip

    @staticmethod
    def out() -> type[TKR[BasePass]]:  # noqa: F821 # fmt: skip
        return TKR[BasePass]  # noqa: F821 # fmt: skip

    @property
    def namespace(self) -> str:
        return "ibmq_worker"


class backend_default_compilation_pass(NamedTuple):
    device_name: TKR[str]  # noqa: F821 # fmt: skip
    optimisation_level: TKR[int] | None = None  # noqa: F821 # fmt: skip

    @staticmethod
    def out() -> type[TKR[BasePass]]:  # noqa: F821 # fmt: skip
        return TKR[BasePass]  # noqa: F821 # fmt: skip

    @property
    def namespace(self) -> str:
        return "ibmq_worker"


class fixed_pass(NamedTuple):
    coupling_map: TKR[Sequence[tuple[int, int]]]  # noqa: F821 # fmt: skip
    optimisation_level: TKR[int] | None = None  # noqa: F821 # fmt: skip

    @staticmethod
    def out() -> type[TKR[BasePass]]:  # noqa: F821 # fmt: skip
        return TKR[BasePass]  # noqa: F821 # fmt: skip

    @property
    def namespace(self) -> str:
        return "ibmq_worker"


class compile(NamedTuple):
    circuit: TKR[Circuit]  # noqa: F821 # fmt: skip
    device_name: TKR[str]  # noqa: F821 # fmt: skip
    optimisation_level: TKR[int] | None = None  # noqa: F821 # fmt: skip

    @staticmethod
    def out() -> type[TKR[Circuit]]:  # noqa: F821 # fmt: skip
        return TKR[Circuit]  # noqa: F821 # fmt: skip

    @property
    def namespace(self) -> str:
        return "ibmq_worker"


class compile_circuit_ibmq(NamedTuple):
    circuit: TKR[Circuit]  # noqa: F821 # fmt: skip
    device_name: TKR[str]  # noqa: F821 # fmt: skip
    optimisation_level: TKR[int] | None = None  # noqa: F821 # fmt: skip

    @staticmethod
    def out() -> type[TKR[Circuit]]:  # noqa: F821 # fmt: skip
        return TKR[Circuit]  # noqa: F821 # fmt: skip

    @property
    def namespace(self) -> str:
        return "ibmq_worker"


class compile_circuits_ibmq(NamedTuple):
    circuits: TKR[list[Circuit]]  # noqa: F821 # fmt: skip
    device_name: TKR[str]  # noqa: F821 # fmt: skip
    optimisation_level: TKR[int] | None = None  # noqa: F821 # fmt: skip

    @staticmethod
    def out() -> type[TKR[list[Circuit]]]:  # noqa: F821 # fmt: skip
        return TKR[list[Circuit]]  # noqa: F821 # fmt: skip

    @property
    def namespace(self) -> str:
        return "ibmq_worker"


class run_circuit(NamedTuple):
    circuit: TKR[Circuit]  # noqa: F821 # fmt: skip
    n_shots: TKR[int]  # noqa: F821 # fmt: skip
    device_name: TKR[str]  # noqa: F821 # fmt: skip

    @staticmethod
    def out() -> type[TKR[BackendResult]]:  # noqa: F821 # fmt: skip
        return TKR[BackendResult]  # noqa: F821 # fmt: skip

    @property
    def namespace(self) -> str:
        return "ibmq_worker"
