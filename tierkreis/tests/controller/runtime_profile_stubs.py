"""Code generated from mpi_worker namespace. Please do not edit."""

from types import NoneType
from typing import NamedTuple

from tierkreis.data.models import TKR


class mpi_rank_info(NamedTuple):
    @staticmethod
    def out() -> type[TKR[str | NoneType]]:  # fmt: skip
        return TKR[str | NoneType]  # fmt: skip

    @property
    def namespace(self) -> str:
        return "mpi_worker"
