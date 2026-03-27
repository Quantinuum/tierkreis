"""Code generated from qulacs_worker namespace. Please do not edit."""

from typing import Literal, NamedTuple, Union
from types import NoneType
from tierkreis.controller.data.models import TKR

type Circuit = Literal["pytket._tket.circuit.Circuit"]
type BackendResult = Literal["pytket.backends.backendresult.BackendResult"]


class get_compiled_circuit(NamedTuple):
    circuit: TKR[Circuit]  # noqa: F821 # fmt: skip
    optimisation_level: TKR[int] | None = None  # noqa: F821 # fmt: skip
    result_type: TKR[str] | None = None  # noqa: F821 # fmt: skip
    gpu_sim: TKR[bool] | None = None  # noqa: F821 # fmt: skip

    @staticmethod
    def out() -> type[TKR[Circuit]]:  # noqa: F821 # fmt: skip
        return TKR[Circuit]  # noqa: F821 # fmt: skip

    @property
    def namespace(self) -> str:
        return "qulacs_worker"


class run_circuit(NamedTuple):
    circuit: TKR[Circuit]  # noqa: F821 # fmt: skip
    n_shots: TKR[int]  # noqa: F821 # fmt: skip
    result_type: TKR[str] | None = None  # noqa: F821 # fmt: skip
    gpu_sim: TKR[bool] | None = None  # noqa: F821 # fmt: skip
    seed: TKR[Union[int, NoneType]] | None = None  # noqa: F821 # fmt: skip

    @staticmethod
    def out() -> type[TKR[BackendResult]]:  # noqa: F821 # fmt: skip
        return TKR[BackendResult]  # noqa: F821 # fmt: skip

    @property
    def namespace(self) -> str:
        return "qulacs_worker"


class run_circuits(NamedTuple):
    circuits: TKR[list[Circuit]]  # noqa: F821 # fmt: skip
    n_shots: TKR[list[int]]  # noqa: F821 # fmt: skip
    result_type: TKR[str] | None = None  # noqa: F821 # fmt: skip
    gpu_sim: TKR[bool] | None = None  # noqa: F821 # fmt: skip
    seed: TKR[Union[int, NoneType]] | None = None  # noqa: F821 # fmt: skip

    @staticmethod
    def out() -> type[TKR[list[BackendResult]]]:  # noqa: F821 # fmt: skip
        return TKR[list[BackendResult]]  # noqa: F821 # fmt: skip

    @property
    def namespace(self) -> str:
        return "qulacs_worker"
