"""Code generated from pytket_worker namespace. Please do not edit."""

from typing import Literal, NamedTuple, Union
from types import NoneType
from tierkreis.controller.data.models import TKR

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
type BackendInfo = Literal["pytket.backends.backendinfo.BackendInfo"]
type Circuit = Literal["pytket._tket.circuit.Circuit"]
type QubitPauliString = Literal["pytket._tket.pauli.QubitPauliString"]
type BasePass = Literal["pytket._tket.passes.BasePass"]
type BackendResult = Literal["pytket.backends.backendresult.BackendResult"]


class get_backend_info(NamedTuple):
    config: TKR[Union[AerConfig, AerStateConfig, AerUnitaryConfig, BraketConfig, QuantinuumConfig, IBMQConfig, IBMQEmulatorConfig, QulacsConfig, SeleneConfig, SelenePlusConfig, HeliosConfig]]  # noqa: F821 # fmt: skip

    @staticmethod
    def out() -> type[TKR[BackendInfo]]:  # noqa: F821 # fmt: skip
        return TKR[BackendInfo]  # noqa: F821 # fmt: skip

    @property
    def namespace(self) -> str:
        return "pytket_worker"


class device_name_from_info(NamedTuple):
    backend_info: TKR[BackendInfo]  # noqa: F821 # fmt: skip

    @staticmethod
    def out() -> type[TKR[Union[str, NoneType]]]:  # noqa: F821 # fmt: skip
        return TKR[Union[str, NoneType]]  # noqa: F821 # fmt: skip

    @property
    def namespace(self) -> str:
        return "pytket_worker"


class compile_using_info(NamedTuple):
    circuit: TKR[Circuit]  # noqa: F821 # fmt: skip
    backend_info: TKR[BackendInfo]  # noqa: F821 # fmt: skip
    config: TKR[Union[AerConfig, AerStateConfig, AerUnitaryConfig, BraketConfig, QuantinuumConfig, IBMQConfig, IBMQEmulatorConfig, QulacsConfig, SeleneConfig, SelenePlusConfig, HeliosConfig]]  # noqa: F821 # fmt: skip
    optimisation_level: TKR[int] | None = None  # noqa: F821 # fmt: skip
    timeout: TKR[int] | None = None  # noqa: F821 # fmt: skip

    @staticmethod
    def out() -> type[TKR[Circuit]]:  # noqa: F821 # fmt: skip
        return TKR[Circuit]  # noqa: F821 # fmt: skip

    @property
    def namespace(self) -> str:
        return "pytket_worker"


class add_measure_all(NamedTuple):
    circuit: TKR[Circuit]  # noqa: F821 # fmt: skip

    @staticmethod
    def out() -> type[TKR[Circuit]]:  # noqa: F821 # fmt: skip
        return TKR[Circuit]  # noqa: F821 # fmt: skip

    @property
    def namespace(self) -> str:
        return "pytket_worker"


class append_pauli_measurement_impl(NamedTuple):
    circuit: TKR[Circuit]  # noqa: F821 # fmt: skip
    pauli_string: TKR[QubitPauliString]  # noqa: F821 # fmt: skip

    @staticmethod
    def out() -> type[TKR[Circuit]]:  # noqa: F821 # fmt: skip
        return TKR[Circuit]  # noqa: F821 # fmt: skip

    @property
    def namespace(self) -> str:
        return "pytket_worker"


class optimise_phase_gadgets(NamedTuple):
    circuit: TKR[Circuit]  # noqa: F821 # fmt: skip

    @staticmethod
    def out() -> type[TKR[Circuit]]:  # noqa: F821 # fmt: skip
        return TKR[Circuit]  # noqa: F821 # fmt: skip

    @property
    def namespace(self) -> str:
        return "pytket_worker"


class apply_pass(NamedTuple):
    circuit: TKR[Circuit]  # noqa: F821 # fmt: skip
    compiler_pass: TKR[BasePass]  # noqa: F821 # fmt: skip

    @staticmethod
    def out() -> type[TKR[Circuit]]:  # noqa: F821 # fmt: skip
        return TKR[Circuit]  # noqa: F821 # fmt: skip

    @property
    def namespace(self) -> str:
        return "pytket_worker"


class compile_generic_with_fixed_pass(NamedTuple):
    circuit: TKR[Union[Circuit, str, bytes]]  # noqa: F821 # fmt: skip
    input_format: TKR[str] | None = None  # noqa: F821 # fmt: skip
    optimisation_level: TKR[int] | None = None  # noqa: F821 # fmt: skip
    gate_set: TKR[Union[list[str], NoneType]] | None = None  # noqa: F821 # fmt: skip
    coupling_map: TKR[Union[list[tuple[int, int]], NoneType]] | None = None  # noqa: F821 # fmt: skip
    output_format: TKR[str] | None = None  # noqa: F821 # fmt: skip
    optimisation_pass: TKR[Union[BasePass, NoneType]] | None = None  # noqa: F821 # fmt: skip

    @staticmethod
    def out() -> type[TKR[Union[Circuit, str, bytes]]]:  # noqa: F821 # fmt: skip
        return TKR[Union[Circuit, str, bytes]]  # noqa: F821 # fmt: skip

    @property
    def namespace(self) -> str:
        return "pytket_worker"


class to_qasm2_str(NamedTuple):
    circuit: TKR[Circuit]  # noqa: F821 # fmt: skip
    header: TKR[str] | None = None  # noqa: F821 # fmt: skip

    @staticmethod
    def out() -> type[TKR[str]]:  # noqa: F821 # fmt: skip
        return TKR[str]  # noqa: F821 # fmt: skip

    @property
    def namespace(self) -> str:
        return "pytket_worker"


class from_qasm2_str(NamedTuple):
    qasm: TKR[str]  # noqa: F821 # fmt: skip

    @staticmethod
    def out() -> type[TKR[Circuit]]:  # noqa: F821 # fmt: skip
        return TKR[Circuit]  # noqa: F821 # fmt: skip

    @property
    def namespace(self) -> str:
        return "pytket_worker"


class to_qir_bytes(NamedTuple):
    circuit: TKR[Circuit]  # noqa: F821 # fmt: skip

    @staticmethod
    def out() -> type[TKR[bytes]]:  # noqa: F821 # fmt: skip
        return TKR[bytes]  # noqa: F821 # fmt: skip

    @property
    def namespace(self) -> str:
        return "pytket_worker"


class from_qir_bytes(NamedTuple):
    qir: TKR[bytes]  # noqa: F821 # fmt: skip

    @staticmethod
    def out() -> type[TKR[Circuit]]:  # noqa: F821 # fmt: skip
        return TKR[Circuit]  # noqa: F821 # fmt: skip

    @property
    def namespace(self) -> str:
        return "pytket_worker"


class expectation(NamedTuple):
    backend_result: TKR[BackendResult]  # noqa: F821 # fmt: skip

    @staticmethod
    def out() -> type[TKR[float]]:  # noqa: F821 # fmt: skip
        return TKR[float]  # noqa: F821 # fmt: skip

    @property
    def namespace(self) -> str:
        return "pytket_worker"


class n_qubits(NamedTuple):
    circuit: TKR[Circuit]  # noqa: F821 # fmt: skip

    @staticmethod
    def out() -> type[TKR[int]]:  # noqa: F821 # fmt: skip
        return TKR[int]  # noqa: F821 # fmt: skip

    @property
    def namespace(self) -> str:
        return "pytket_worker"


class backend_result_to_dict(NamedTuple):
    backend_result: TKR[BackendResult]  # noqa: F821 # fmt: skip

    @staticmethod
    def out() -> type[TKR[dict[str, list[str]]]]:  # noqa: F821 # fmt: skip
        return TKR[dict[str, list[str]]]  # noqa: F821 # fmt: skip

    @property
    def namespace(self) -> str:
        return "pytket_worker"


class backend_result_from_dict(NamedTuple):
    data: TKR[dict[str, list[str]]]  # noqa: F821 # fmt: skip

    @staticmethod
    def out() -> type[TKR[BackendResult]]:  # noqa: F821 # fmt: skip
        return TKR[BackendResult]  # noqa: F821 # fmt: skip

    @property
    def namespace(self) -> str:
        return "pytket_worker"
