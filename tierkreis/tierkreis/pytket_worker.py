"""Code generated from pytket_worker namespace. Please do not edit."""

# ruff: noqa: F821
from types import NoneType
from typing import NamedTuple

from tierkreis.controller.data.models import TKR, OpaqueType


class get_backend_info(NamedTuple):
    config: TKR[
        OpaqueType["quantinuum_schemas.models.backend_config.AerConfig"]
        | OpaqueType["quantinuum_schemas.models.backend_config.AerStateConfig"]
        | OpaqueType["quantinuum_schemas.models.backend_config.AerUnitaryConfig"]
        | OpaqueType["quantinuum_schemas.models.backend_config.BraketConfig"]
        | OpaqueType["quantinuum_schemas.models.backend_config.QuantinuumConfig"]
        | OpaqueType["quantinuum_schemas.models.backend_config.IBMQConfig"]
        | OpaqueType["quantinuum_schemas.models.backend_config.IBMQEmulatorConfig"]
        | OpaqueType["quantinuum_schemas.models.backend_config.QulacsConfig"]
        | OpaqueType["quantinuum_schemas.models.backend_config.SeleneConfig"]
        | OpaqueType["quantinuum_schemas.models.backend_config.SelenePlusConfig"]
        | OpaqueType["quantinuum_schemas.models.backend_config.HeliosConfig"]
    ]

    @staticmethod
    def out() -> type[TKR[OpaqueType["pytket.backends.backendinfo.BackendInfo"]]]:
        return TKR[OpaqueType["pytket.backends.backendinfo.BackendInfo"]]

    @property
    def namespace(self) -> str:
        return "pytket_worker"


class device_name_from_info(NamedTuple):
    backend_info: TKR[OpaqueType["pytket.backends.backendinfo.BackendInfo"]]

    @staticmethod
    def out() -> type[TKR[str | NoneType]]:
        return TKR[str | NoneType]

    @property
    def namespace(self) -> str:
        return "pytket_worker"


class compile_using_info(NamedTuple):
    circuit: TKR[OpaqueType["pytket._tket.circuit.Circuit"]]
    backend_info: TKR[OpaqueType["pytket.backends.backendinfo.BackendInfo"]]
    config: TKR[
        OpaqueType["quantinuum_schemas.models.backend_config.AerConfig"]
        | OpaqueType["quantinuum_schemas.models.backend_config.AerStateConfig"]
        | OpaqueType["quantinuum_schemas.models.backend_config.AerUnitaryConfig"]
        | OpaqueType["quantinuum_schemas.models.backend_config.BraketConfig"]
        | OpaqueType["quantinuum_schemas.models.backend_config.QuantinuumConfig"]
        | OpaqueType["quantinuum_schemas.models.backend_config.IBMQConfig"]
        | OpaqueType["quantinuum_schemas.models.backend_config.IBMQEmulatorConfig"]
        | OpaqueType["quantinuum_schemas.models.backend_config.QulacsConfig"]
        | OpaqueType["quantinuum_schemas.models.backend_config.SeleneConfig"]
        | OpaqueType["quantinuum_schemas.models.backend_config.SelenePlusConfig"]
        | OpaqueType["quantinuum_schemas.models.backend_config.HeliosConfig"]
    ]
    optimisation_level: TKR[int] | None = None
    timeout: TKR[int] | None = None

    @staticmethod
    def out() -> type[TKR[OpaqueType["pytket._tket.circuit.Circuit"]]]:
        return TKR[OpaqueType["pytket._tket.circuit.Circuit"]]

    @property
    def namespace(self) -> str:
        return "pytket_worker"


class add_measure_all(NamedTuple):
    circuit: TKR[OpaqueType["pytket._tket.circuit.Circuit"]]

    @staticmethod
    def out() -> type[TKR[OpaqueType["pytket._tket.circuit.Circuit"]]]:
        return TKR[OpaqueType["pytket._tket.circuit.Circuit"]]

    @property
    def namespace(self) -> str:
        return "pytket_worker"


class append_pauli_measurement_impl(NamedTuple):
    circuit: TKR[OpaqueType["pytket._tket.circuit.Circuit"]]
    pauli_string: TKR[OpaqueType["pytket._tket.pauli.QubitPauliString"]]

    @staticmethod
    def out() -> type[TKR[OpaqueType["pytket._tket.circuit.Circuit"]]]:
        return TKR[OpaqueType["pytket._tket.circuit.Circuit"]]

    @property
    def namespace(self) -> str:
        return "pytket_worker"


class optimise_phase_gadgets(NamedTuple):
    circuit: TKR[OpaqueType["pytket._tket.circuit.Circuit"]]

    @staticmethod
    def out() -> type[TKR[OpaqueType["pytket._tket.circuit.Circuit"]]]:
        return TKR[OpaqueType["pytket._tket.circuit.Circuit"]]

    @property
    def namespace(self) -> str:
        return "pytket_worker"


class apply_pass(NamedTuple):
    circuit: TKR[OpaqueType["pytket._tket.circuit.Circuit"]]
    compiler_pass: TKR[OpaqueType["pytket._tket.passes.BasePass"]]

    @staticmethod
    def out() -> type[TKR[OpaqueType["pytket._tket.circuit.Circuit"]]]:
        return TKR[OpaqueType["pytket._tket.circuit.Circuit"]]

    @property
    def namespace(self) -> str:
        return "pytket_worker"


class compile_generic_with_fixed_pass(NamedTuple):
    circuit: TKR[OpaqueType["pytket._tket.circuit.Circuit"] | str | bytes]
    input_format: TKR[str] | None = None
    optimisation_level: TKR[int] | None = None
    gate_set: TKR[list[str] | NoneType] | None = None
    coupling_map: TKR[list[tuple[int, int]] | NoneType] | None = None
    output_format: TKR[str] | None = None
    optimisation_pass: (
        TKR[OpaqueType["pytket._tket.passes.BasePass"] | NoneType] | None
    ) = None

    @staticmethod
    def out() -> type[TKR[OpaqueType["pytket._tket.circuit.Circuit"] | str | bytes]]:
        return TKR[OpaqueType["pytket._tket.circuit.Circuit"] | str | bytes]

    @property
    def namespace(self) -> str:
        return "pytket_worker"


class to_qasm2_str(NamedTuple):
    circuit: TKR[OpaqueType["pytket._tket.circuit.Circuit"]]
    header: TKR[str] | None = None

    @staticmethod
    def out() -> type[TKR[str]]:
        return TKR[str]

    @property
    def namespace(self) -> str:
        return "pytket_worker"


class from_qasm2_str(NamedTuple):
    qasm: TKR[str]

    @staticmethod
    def out() -> type[TKR[OpaqueType["pytket._tket.circuit.Circuit"]]]:
        return TKR[OpaqueType["pytket._tket.circuit.Circuit"]]

    @property
    def namespace(self) -> str:
        return "pytket_worker"


class to_qir_bytes(NamedTuple):
    circuit: TKR[OpaqueType["pytket._tket.circuit.Circuit"]]

    @staticmethod
    def out() -> type[TKR[bytes]]:
        return TKR[bytes]

    @property
    def namespace(self) -> str:
        return "pytket_worker"


class from_qir_bytes(NamedTuple):
    qir: TKR[bytes]

    @staticmethod
    def out() -> type[TKR[OpaqueType["pytket._tket.circuit.Circuit"]]]:
        return TKR[OpaqueType["pytket._tket.circuit.Circuit"]]

    @property
    def namespace(self) -> str:
        return "pytket_worker"


class expectation(NamedTuple):
    backend_result: TKR[OpaqueType["pytket.backends.backendresult.BackendResult"]]

    @staticmethod
    def out() -> type[TKR[float]]:
        return TKR[float]

    @property
    def namespace(self) -> str:
        return "pytket_worker"


class n_qubits(NamedTuple):
    circuit: TKR[OpaqueType["pytket._tket.circuit.Circuit"]]

    @staticmethod
    def out() -> type[TKR[int]]:
        return TKR[int]

    @property
    def namespace(self) -> str:
        return "pytket_worker"


class backend_result_to_dict(NamedTuple):
    backend_result: TKR[OpaqueType["pytket.backends.backendresult.BackendResult"]]

    @staticmethod
    def out() -> type[TKR[dict[str, list[str]]]]:
        return TKR[dict[str, list[str]]]

    @property
    def namespace(self) -> str:
        return "pytket_worker"


class backend_result_from_dict(NamedTuple):
    data: TKR[dict[str, list[str]]]

    @staticmethod
    def out() -> type[TKR[OpaqueType["pytket.backends.backendresult.BackendResult"]]]:
        return TKR[OpaqueType["pytket.backends.backendresult.BackendResult"]]

    @property
    def namespace(self) -> str:
        return "pytket_worker"
