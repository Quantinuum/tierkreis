"""Code generated from multiple_outputs_worker namespace. Please do not edit."""

from typing import NamedTuple, Protocol
from tierkreis.controller.data.models import TKR
from tierkreis.controller.data.types import Struct


class OpaquePoint(Struct, Protocol):
    x: float  # noqa: F821 # fmt: skip
    y: float  # noqa: F821 # fmt: skip
    z: float  # noqa: F821 # fmt: skip


class Point(NamedTuple):
    x: TKR[float]  # noqa: F821 # fmt: skip
    y: TKR[float]  # noqa: F821 # fmt: skip
    z: TKR[float]  # noqa: F821 # fmt: skip


class new_opaque_point(NamedTuple):
    x: TKR[float]  # noqa: F821 # fmt: skip
    y: TKR[float]  # noqa: F821 # fmt: skip
    z: TKR[float]  # noqa: F821 # fmt: skip

    @staticmethod
    def out() -> type[TKR[OpaquePoint]]:  # noqa: F821 # fmt: skip
        return TKR[OpaquePoint]  # noqa: F821 # fmt: skip

    @property
    def namespace(self) -> str:
        return "multiple_outputs_worker"


class new_point(NamedTuple):
    x: TKR[float]  # noqa: F821 # fmt: skip
    y: TKR[float]  # noqa: F821 # fmt: skip
    z: TKR[float]  # noqa: F821 # fmt: skip

    @staticmethod
    def out() -> type[Point]:  # noqa: F821 # fmt: skip
        return Point  # noqa: F821 # fmt: skip

    @property
    def namespace(self) -> str:
        return "multiple_outputs_worker"
