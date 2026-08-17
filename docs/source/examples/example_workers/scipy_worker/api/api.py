"""Code generated from scipy_worker namespace. Please do not edit."""

# ruff: noqa: F821
from typing import NamedTuple, Protocol, Union

from tierkreis.controller.data.models import TKR, OpaqueType
from tierkreis.controller.data.types import Struct


class PointedArray(Struct, Protocol):
    a: OpaqueType["numpy.ndarray"]
    p: int


class add_point(NamedTuple):
    a: TKR[OpaqueType["numpy.ndarray"]]
    p: TKR[int]

    @staticmethod
    def out() -> type[TKR[PointedArray]]:
        return TKR[PointedArray]

    @property
    def namespace(self) -> str:
        return "scipy_worker"


class eval_point(NamedTuple):
    pa: TKR[PointedArray]

    @staticmethod
    def out() -> type[TKR[float]]:
        return TKR[float]

    @property
    def namespace(self) -> str:
        return "scipy_worker"


class linspace(NamedTuple):
    start: TKR[float]
    stop: TKR[float]
    num: TKR[int] | None = None

    @staticmethod
    def out() -> type[TKR[OpaqueType["numpy.ndarray"]]]:
        return TKR[OpaqueType["numpy.ndarray"]]

    @property
    def namespace(self) -> str:
        return "scipy_worker"


class transpose(NamedTuple):
    a: TKR[OpaqueType["numpy.ndarray"]]

    @staticmethod
    def out() -> type[TKR[OpaqueType["numpy.ndarray"]]]:
        return TKR[OpaqueType["numpy.ndarray"]]

    @property
    def namespace(self) -> str:
        return "scipy_worker"


class reshape(NamedTuple):
    a: TKR[OpaqueType["numpy.ndarray"]]
    shape: TKR[Union[int, list[int]]]

    @staticmethod
    def out() -> type[TKR[OpaqueType["numpy.ndarray"]]]:
        return TKR[OpaqueType["numpy.ndarray"]]

    @property
    def namespace(self) -> str:
        return "scipy_worker"
