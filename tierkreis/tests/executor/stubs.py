"""Code generated from slurm_mpi_worker namespace. Please do not edit."""

from typing import NamedTuple, Union
from types import NoneType
from tierkreis.controller.data.models import TKR


class mpi_rank_info(NamedTuple):
    @staticmethod
    def out() -> type[TKR[Union[str, NoneType]]]:  # noqa: F821 # fmt: skip
        return TKR[Union[str, NoneType]]  # noqa: F821 # fmt: skip

    @property
    def namespace(self) -> str:
        return "slurm_mpi_worker"


class mpi_fail(NamedTuple):
    @staticmethod
    def out() -> type[TKR[NoneType]]:  # noqa: F821 # fmt: skip
        return TKR[NoneType]  # noqa: F821 # fmt: skip

    @property
    def namespace(self) -> str:
        return "slurm_mpi_worker"
