"""Code generated from nexus_worker namespace. Please do not edit."""

# ruff: noqa: F821
from typing import NamedTuple, Union
from types import NoneType
from tierkreis.controller.data.models import TKR, OpaqueType


class upload_circuit(NamedTuple):
    project_name: TKR[str]
    circ: TKR[OpaqueType["pytket._tket.circuit.Circuit"]]

    @staticmethod
    def out() -> type[
        TKR[
            Union[
                OpaqueType["qnexus.models.references.CircuitRef"],
                OpaqueType["qnexus.models.references.HUGRRef"],
                OpaqueType["qnexus.models.references.QIRRef"],
            ]
        ]
    ]:
        return TKR[
            Union[
                OpaqueType["qnexus.models.references.CircuitRef"],
                OpaqueType["qnexus.models.references.HUGRRef"],
                OpaqueType["qnexus.models.references.QIRRef"],
            ]
        ]

    @property
    def namespace(self) -> str:
        return "nexus_worker"


class start_execute_job(NamedTuple):
    project_name: TKR[str]
    job_name: TKR[str]
    circuits: TKR[
        list[
            Union[
                OpaqueType["qnexus.models.references.CircuitRef"],
                OpaqueType["qnexus.models.references.HUGRRef"],
                OpaqueType["qnexus.models.references.QIRRef"],
            ]
        ]
    ]
    n_shots: TKR[list[int]]
    backend_config: TKR[
        Union[
            OpaqueType["quantinuum_schemas.models.backend_config.AerConfig"],
            OpaqueType["quantinuum_schemas.models.backend_config.AerStateConfig"],
            OpaqueType["quantinuum_schemas.models.backend_config.AerUnitaryConfig"],
            OpaqueType["quantinuum_schemas.models.backend_config.BraketConfig"],
            OpaqueType["quantinuum_schemas.models.backend_config.QuantinuumConfig"],
            OpaqueType["quantinuum_schemas.models.backend_config.IBMQConfig"],
            OpaqueType["quantinuum_schemas.models.backend_config.IBMQEmulatorConfig"],
            OpaqueType["quantinuum_schemas.models.backend_config.QulacsConfig"],
            OpaqueType["quantinuum_schemas.models.backend_config.SeleneConfig"],
            OpaqueType["quantinuum_schemas.models.backend_config.SelenePlusConfig"],
            OpaqueType["quantinuum_schemas.models.backend_config.HeliosConfig"],
        ]
    ]

    @staticmethod
    def out() -> type[TKR[OpaqueType["qnexus.models.references.ExecuteJobRef"]]]:
        return TKR[OpaqueType["qnexus.models.references.ExecuteJobRef"]]

    @property
    def namespace(self) -> str:
        return "nexus_worker"


class start_single_job(NamedTuple):
    project_name: TKR[str]
    job_name: TKR[str]
    circuit: TKR[
        Union[
            OpaqueType["qnexus.models.references.CircuitRef"],
            OpaqueType["qnexus.models.references.HUGRRef"],
            OpaqueType["qnexus.models.references.QIRRef"],
        ]
    ]
    n_shots: TKR[int]
    backend_config: TKR[
        Union[
            OpaqueType["quantinuum_schemas.models.backend_config.AerConfig"],
            OpaqueType["quantinuum_schemas.models.backend_config.AerStateConfig"],
            OpaqueType["quantinuum_schemas.models.backend_config.AerUnitaryConfig"],
            OpaqueType["quantinuum_schemas.models.backend_config.BraketConfig"],
            OpaqueType["quantinuum_schemas.models.backend_config.QuantinuumConfig"],
            OpaqueType["quantinuum_schemas.models.backend_config.IBMQConfig"],
            OpaqueType["quantinuum_schemas.models.backend_config.IBMQEmulatorConfig"],
            OpaqueType["quantinuum_schemas.models.backend_config.QulacsConfig"],
            OpaqueType["quantinuum_schemas.models.backend_config.SeleneConfig"],
            OpaqueType["quantinuum_schemas.models.backend_config.SelenePlusConfig"],
            OpaqueType["quantinuum_schemas.models.backend_config.HeliosConfig"],
        ]
    ]

    @staticmethod
    def out() -> type[TKR[OpaqueType["qnexus.models.references.ExecuteJobRef"]]]:
        return TKR[OpaqueType["qnexus.models.references.ExecuteJobRef"]]

    @property
    def namespace(self) -> str:
        return "nexus_worker"


class is_running(NamedTuple):
    execute_ref: TKR[OpaqueType["qnexus.models.references.ExecuteJobRef"]]

    @staticmethod
    def out() -> type[TKR[bool]]:
        return TKR[bool]

    @property
    def namespace(self) -> str:
        return "nexus_worker"


class get_results(NamedTuple):
    execute_ref: TKR[OpaqueType["qnexus.models.references.ExecuteJobRef"]]

    @staticmethod
    def out() -> type[
        TKR[list[OpaqueType["pytket.backends.backendresult.BackendResult"]]]
    ]:
        return TKR[list[OpaqueType["pytket.backends.backendresult.BackendResult"]]]

    @property
    def namespace(self) -> str:
        return "nexus_worker"


class upload_hugr(NamedTuple):
    hugr_package: TKR[OpaqueType["hugr.package.Package"]]
    project_name: TKR[str]
    name: TKR[Union[str, NoneType]] | None = None

    @staticmethod
    def out() -> type[TKR[OpaqueType["qnexus.models.references.HUGRRef"]]]:
        return TKR[OpaqueType["qnexus.models.references.HUGRRef"]]

    @property
    def namespace(self) -> str:
        return "nexus_worker"


class cost(NamedTuple):
    hugr_ref: TKR[
        Union[
            OpaqueType["qnexus.models.references.HUGRRef"],
            list[OpaqueType["qnexus.models.references.HUGRRef"]],
        ]
    ]
    n_shots: TKR[int]
    project_name: TKR[str]

    @staticmethod
    def out() -> type[TKR[float]]:
        return TKR[float]

    @property
    def namespace(self) -> str:
        return "nexus_worker"
