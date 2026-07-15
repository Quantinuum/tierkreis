"""Code generated from mpi_worker namespace. Please do not edit."""

from typing import NamedTuple, Union
from types import NoneType
from tierkreis.controller.data.models import TKR


class mpi_rank_info(NamedTuple):
    @staticmethod
    def out() -> type[TKR[Union[str, NoneType]]]:  # noqa: F821 # fmt: skip
        return TKR[Union[str, NoneType]]  # noqa: F821 # fmt: skip

    @property
    def namespace(self) -> str:
        return "mpi_worker"


class mpi_rank_info_with_input(NamedTuple):
    value: TKR[str]  # noqa: F821 # fmt: skip

    @staticmethod
    def out() -> type[TKR[Union[str, NoneType]]]:  # noqa: F821 # fmt: skip
        return TKR[Union[str, NoneType]]  # noqa: F821 # fmt: skip

    @property
    def namespace(self) -> str:
        return "mpi_worker"


class mpi_fail(NamedTuple):
    @staticmethod
    def out() -> type[TKR[NoneType]]:  # noqa: F821 # fmt: skip
        return TKR[NoneType]  # noqa: F821 # fmt: skip

    @property
    def namespace(self) -> str:
        return "mpi_worker"
