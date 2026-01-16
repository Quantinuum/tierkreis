"""Code generated from mc_worker namespace. Please do not edit."""

from typing import NamedTuple
from tierkreis.controller.data.models import TKR


class IsingStats(NamedTuple):
    mean_energy: TKR[float]  # noqa: F821 # fmt: skip
    mean_magnetization: TKR[float]  # noqa: F821 # fmt: skip


class ising_mcmc_samples(NamedTuple):
    M: TKR[int]  # noqa: F821 # fmt: skip
    J: TKR[list[list[float]]]  # noqa: F821 # fmt: skip
    h: TKR[list[float]]  # noqa: F821 # fmt: skip
    seed: TKR[int]  # noqa: F821 # fmt: skip
    temperature: TKR[float] | None = None  # noqa: F821 # fmt: skip

    @staticmethod
    def out() -> type[TKR[list[str]]]:  # noqa: F821 # fmt: skip
        return TKR[list[str]]  # noqa: F821 # fmt: skip

    @property
    def namespace(self) -> str:
        return "mc_worker"


class ising_expected_stats(NamedTuple):
    samples: TKR[list[str]]  # noqa: F821 # fmt: skip
    J: TKR[list[list[float]]]  # noqa: F821 # fmt: skip
    h: TKR[list[float]]  # noqa: F821 # fmt: skip

    @staticmethod
    def out() -> type[IsingStats]:  # noqa: F821 # fmt: skip
        return IsingStats  # noqa: F821 # fmt: skip

    @property
    def namespace(self) -> str:
        return "mc_worker"

