"""Code generated from nexus_worker namespace. Please do not edit."""

from typing import NamedTuple, Union
from tierkreis.controller.data.models import TKR, OpaqueType


class upload_circuit(NamedTuple):
    project_name: TKR[str]  # noqa: F821 # fmt: skip
    circ: TKR[OpaqueType["pytket._tket.circuit.Circuit"]]  # noqa: F821 # fmt: skip

    @staticmethod
    def out() -> type[TKR[Union[OpaqueType["qnexus.models.references.CircuitRef"], OpaqueType["qnexus.models.references.HUGRRef"], OpaqueType["qnexus.models.references.QIRRef"]]]]:  # noqa: F821 # fmt: skip
        return TKR[Union[OpaqueType["qnexus.models.references.CircuitRef"], OpaqueType["qnexus.models.references.HUGRRef"], OpaqueType["qnexus.models.references.QIRRef"]]]  # noqa: F821 # fmt: skip

    @property
    def namespace(self) -> str:
        return "nexus_worker"


class start_execute_job(NamedTuple):
    project_name: TKR[str]  # noqa: F821 # fmt: skip
    job_name: TKR[str]  # noqa: F821 # fmt: skip
    circuits: TKR[list[Union[OpaqueType["qnexus.models.references.CircuitRef"], OpaqueType["qnexus.models.references.HUGRRef"], OpaqueType["qnexus.models.references.QIRRef"]]]]  # noqa: F821 # fmt: skip
    n_shots: TKR[list[int]]  # noqa: F821 # fmt: skip
    backend_config: TKR[Union[OpaqueType["quantinuum_schemas.models.backend_config.AerConfig"], OpaqueType["quantinuum_schemas.models.backend_config.AerStateConfig"], OpaqueType["quantinuum_schemas.models.backend_config.AerUnitaryConfig"], OpaqueType["quantinuum_schemas.models.backend_config.BraketConfig"], OpaqueType["quantinuum_schemas.models.backend_config.QuantinuumConfig"], OpaqueType["quantinuum_schemas.models.backend_config.IBMQConfig"], OpaqueType["quantinuum_schemas.models.backend_config.IBMQEmulatorConfig"], OpaqueType["quantinuum_schemas.models.backend_config.QulacsConfig"], OpaqueType["quantinuum_schemas.models.backend_config.SeleneConfig"], OpaqueType["quantinuum_schemas.models.backend_config.SelenePlusConfig"], OpaqueType["quantinuum_schemas.models.backend_config.HeliosConfig"]]]  # noqa: F821 # fmt: skip

    @staticmethod
    def out() -> type[TKR[OpaqueType["qnexus.models.references.ExecuteJobRef"]]]:  # noqa: F821 # fmt: skip
        return TKR[OpaqueType["qnexus.models.references.ExecuteJobRef"]]  # noqa: F821 # fmt: skip

    @property
    def namespace(self) -> str:
        return "nexus_worker"


class is_running(NamedTuple):
    execute_ref: TKR[OpaqueType["qnexus.models.references.ExecuteJobRef"]]  # noqa: F821 # fmt: skip

    @staticmethod
    def out() -> type[TKR[bool]]:  # noqa: F821 # fmt: skip
        return TKR[bool]  # noqa: F821 # fmt: skip

    @property
    def namespace(self) -> str:
        return "nexus_worker"


class get_results(NamedTuple):
    execute_ref: TKR[OpaqueType["qnexus.models.references.ExecuteJobRef"]]  # noqa: F821 # fmt: skip

    @staticmethod
    def out() -> type[TKR[list[OpaqueType["pytket.backends.backendresult.BackendResult"]]]]:  # noqa: F821 # fmt: skip
        return TKR[list[OpaqueType["pytket.backends.backendresult.BackendResult"]]]  # noqa: F821 # fmt: skip

    @property
    def namespace(self) -> str:
        return "nexus_worker"


class check_status(NamedTuple):
    execute_ref: TKR[OpaqueType["qnexus.models.references.ExecuteJobRef"]]  # noqa: F821 # fmt: skip

    @staticmethod
    def out() -> type[TKR[str]]:  # noqa: F821 # fmt: skip
        return TKR[str]  # noqa: F821 # fmt: skip

    @property
    def namespace(self) -> str:
        return "nexus_worker"


class submit(NamedTuple):
    circuits: TKR[list[OpaqueType["pytket._tket.circuit.Circuit"]]]  # noqa: F821 # fmt: skip
    n_shots: TKR[int]  # noqa: F821 # fmt: skip

    @staticmethod
    def out() -> type[TKR[OpaqueType["qnexus.models.references.ExecuteJobRef"]]]:  # noqa: F821 # fmt: skip
        return TKR[OpaqueType["qnexus.models.references.ExecuteJobRef"]]  # noqa: F821 # fmt: skip

    @property
    def namespace(self) -> str:
        return "nexus_worker"
