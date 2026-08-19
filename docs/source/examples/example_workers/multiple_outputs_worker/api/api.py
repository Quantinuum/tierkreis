"""Code generated from multiple_outputs_worker namespace. Please do not edit."""

from typing import NamedTuple, Protocol

from tierkreis.controller.data.models import TKR
from tierkreis.controller.data.types import Struct


class OpaquePoint(Struct, Protocol):
    x: float
    y: float
    z: float


class Point(NamedTuple):
    x: TKR[float]
    y: TKR[float]
    z: TKR[float]


class new_opaque_point(NamedTuple):
    x: TKR[float]
    y: TKR[float]
    z: TKR[float]

    @staticmethod
    def out() -> type[TKR[OpaquePoint]]:
        return TKR[OpaquePoint]

    @property
    def namespace(self) -> str:
        return "multiple_outputs_worker"


class new_point(NamedTuple):
    x: TKR[float]
    y: TKR[float]
    z: TKR[float]

    @staticmethod
    def out() -> type[Point]:
        return Point

    @property
    def namespace(self) -> str:
        return "multiple_outputs_worker"
