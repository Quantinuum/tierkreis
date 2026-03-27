"""Code generated from quantinuum_worker namespace. Please do not edit."""

from typing import Literal, NamedTuple
from tierkreis.controller.data.models import TKR

type BackendInfo = Literal["pytket.backends.backendinfo.BackendInfo"]
type Circuit = Literal["pytket._tket.circuit.Circuit"]
type BasePass = Literal["pytket._tket.passes.BasePass"]
type BackendResult = Literal["pytket.backends.backendresult.BackendResult"]


class get_backend_info(NamedTuple):
    device_name: TKR[str]  # noqa: F821 # fmt: skip

    @staticmethod
    def out() -> type[TKR[BackendInfo]]:  # noqa: F821 # fmt: skip
        return TKR[BackendInfo]  # noqa: F821 # fmt: skip

    @property
    def namespace(self) -> str:
        return "quantinuum_worker"


class compile_using_info(NamedTuple):
    circuit: TKR[Circuit]  # noqa: F821 # fmt: skip
    backend_info: TKR[BackendInfo]  # noqa: F821 # fmt: skip
    optimisation_level: TKR[int] | None = None  # noqa: F821 # fmt: skip

    @staticmethod
    def out() -> type[TKR[Circuit]]:  # noqa: F821 # fmt: skip
        return TKR[Circuit]  # noqa: F821 # fmt: skip

    @property
    def namespace(self) -> str:
        return "quantinuum_worker"


class backend_pass_from_info(NamedTuple):
    backend_info: TKR[BackendInfo]  # noqa: F821 # fmt: skip
    optimisation_level: TKR[int] | None = None  # noqa: F821 # fmt: skip

    @staticmethod
    def out() -> type[TKR[BasePass]]:  # noqa: F821 # fmt: skip
        return TKR[BasePass]  # noqa: F821 # fmt: skip

    @property
    def namespace(self) -> str:
        return "quantinuum_worker"


class fixed_pass(NamedTuple):
    @staticmethod
    def out() -> type[TKR[BasePass]]:  # noqa: F821 # fmt: skip
        return TKR[BasePass]  # noqa: F821 # fmt: skip

    @property
    def namespace(self) -> str:
        return "quantinuum_worker"


class compile_circuit_quantinuum(NamedTuple):
    circuit: TKR[Circuit]  # noqa: F821 # fmt: skip

    @staticmethod
    def out() -> type[TKR[Circuit]]:  # noqa: F821 # fmt: skip
        return TKR[Circuit]  # noqa: F821 # fmt: skip

    @property
    def namespace(self) -> str:
        return "quantinuum_worker"


class compile_circuits_quantinuum(NamedTuple):
    circuits: TKR[list[Circuit]]  # noqa: F821 # fmt: skip

    @staticmethod
    def out() -> type[TKR[list[Circuit]]]:  # noqa: F821 # fmt: skip
        return TKR[list[Circuit]]  # noqa: F821 # fmt: skip

    @property
    def namespace(self) -> str:
        return "quantinuum_worker"


class run_circuit(NamedTuple):
    circuit: TKR[Circuit]  # noqa: F821 # fmt: skip
    n_shots: TKR[int]  # noqa: F821 # fmt: skip
    device_name: TKR[str]  # noqa: F821 # fmt: skip

    @staticmethod
    def out() -> type[TKR[BackendResult]]:  # noqa: F821 # fmt: skip
        return TKR[BackendResult]  # noqa: F821 # fmt: skip

    @property
    def namespace(self) -> str:
        return "quantinuum_worker"
