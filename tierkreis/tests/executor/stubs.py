"""Code generated from slurm_mpi_worker namespace. Please do not edit."""

from types import NoneType
from typing import NamedTuple, Union

from tierkreis.controller.data.models import TKR


class mpi_rank_info(NamedTuple):
    @staticmethod
    def out() -> type[TKR[str | NoneType]]:  # fmt: skip
        return TKR[Union[str, NoneType]]  # fmt: skip

    @property
    def namespace(self) -> str:
        return "mpi_worker"


class mpi_fail(NamedTuple):
    @staticmethod
    def out() -> type[TKR[NoneType]]:  # fmt: skip
        return TKR[NoneType]  # fmt: skip

    @property
    def namespace(self) -> str:
        return "mpi_worker"
