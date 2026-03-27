"""Code generated from nexus_worker namespace. Please do not edit."""

from typing import Literal, NamedTuple, Union
from tierkreis.controller.data.models import TKR

type Circuit = Literal["pytket._tket.circuit.Circuit"]
type CircuitRef = Literal["qnexus.models.references.CircuitRef"]
type HUGRRef = Literal["qnexus.models.references.HUGRRef"]
type QIRRef = Literal["qnexus.models.references.QIRRef"]
type AerConfig = Literal["quantinuum_schemas.models.backend_config.AerConfig"]
type AerStateConfig = Literal["quantinuum_schemas.models.backend_config.AerStateConfig"]
type AerUnitaryConfig = Literal[
    "quantinuum_schemas.models.backend_config.AerUnitaryConfig"
]
type BraketConfig = Literal["quantinuum_schemas.models.backend_config.BraketConfig"]
type QuantinuumConfig = Literal[
    "quantinuum_schemas.models.backend_config.QuantinuumConfig"
]
type IBMQConfig = Literal["quantinuum_schemas.models.backend_config.IBMQConfig"]
type IBMQEmulatorConfig = Literal[
    "quantinuum_schemas.models.backend_config.IBMQEmulatorConfig"
]
type QulacsConfig = Literal["quantinuum_schemas.models.backend_config.QulacsConfig"]
type SeleneConfig = Literal["quantinuum_schemas.models.backend_config.SeleneConfig"]
type SelenePlusConfig = Literal[
    "quantinuum_schemas.models.backend_config.SelenePlusConfig"
]
type HeliosConfig = Literal["quantinuum_schemas.models.backend_config.HeliosConfig"]
type ExecuteJobRef = Literal["qnexus.models.references.ExecuteJobRef"]
type BackendResult = Literal["pytket.backends.backendresult.BackendResult"]


class upload_circuit(NamedTuple):
    project_name: TKR[str]  # noqa: F821 # fmt: skip
    circ: TKR[Circuit]  # noqa: F821 # fmt: skip

    @staticmethod
    def out() -> type[TKR[Union[CircuitRef, HUGRRef, QIRRef]]]:  # noqa: F821 # fmt: skip
        return TKR[Union[CircuitRef, HUGRRef, QIRRef]]  # noqa: F821 # fmt: skip

    @property
    def namespace(self) -> str:
        return "nexus_worker"


class start_execute_job(NamedTuple):
    project_name: TKR[str]  # noqa: F821 # fmt: skip
    job_name: TKR[str]  # noqa: F821 # fmt: skip
    circuits: TKR[list[Union[CircuitRef, HUGRRef, QIRRef]]]  # noqa: F821 # fmt: skip
    n_shots: TKR[list[int]]  # noqa: F821 # fmt: skip
    backend_config: TKR[Union[AerConfig, AerStateConfig, AerUnitaryConfig, BraketConfig, QuantinuumConfig, IBMQConfig, IBMQEmulatorConfig, QulacsConfig, SeleneConfig, SelenePlusConfig, HeliosConfig]]  # noqa: F821 # fmt: skip

    @staticmethod
    def out() -> type[TKR[ExecuteJobRef]]:  # noqa: F821 # fmt: skip
        return TKR[ExecuteJobRef]  # noqa: F821 # fmt: skip

    @property
    def namespace(self) -> str:
        return "nexus_worker"


class is_running(NamedTuple):
    execute_ref: TKR[ExecuteJobRef]  # noqa: F821 # fmt: skip

    @staticmethod
    def out() -> type[TKR[bool]]:  # noqa: F821 # fmt: skip
        return TKR[bool]  # noqa: F821 # fmt: skip

    @property
    def namespace(self) -> str:
        return "nexus_worker"


class get_results(NamedTuple):
    execute_ref: TKR[ExecuteJobRef]  # noqa: F821 # fmt: skip

    @staticmethod
    def out() -> type[TKR[list[BackendResult]]]:  # noqa: F821 # fmt: skip
        return TKR[list[BackendResult]]  # noqa: F821 # fmt: skip

    @property
    def namespace(self) -> str:
        return "nexus_worker"


class check_status(NamedTuple):
    execute_ref: TKR[ExecuteJobRef]  # noqa: F821 # fmt: skip

    @staticmethod
    def out() -> type[TKR[str]]:  # noqa: F821 # fmt: skip
        return TKR[str]  # noqa: F821 # fmt: skip

    @property
    def namespace(self) -> str:
        return "nexus_worker"


class submit(NamedTuple):
    circuits: TKR[list[Circuit]]  # noqa: F821 # fmt: skip
    n_shots: TKR[int]  # noqa: F821 # fmt: skip

    @staticmethod
    def out() -> type[TKR[ExecuteJobRef]]:  # noqa: F821 # fmt: skip
        return TKR[ExecuteJobRef]  # noqa: F821 # fmt: skip

    @property
    def namespace(self) -> str:
        return "nexus_worker"
