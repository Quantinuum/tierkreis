"""Code generated from scipy_worker namespace. Please do not edit."""

from typing import NamedTuple, Protocol, Union
from tierkreis.controller.data.models import TKR, OpaqueType
from tierkreis.controller.data.types import Struct


class PointedArray(Struct, Protocol):
    a: OpaqueType["numpy.ndarray"]  # noqa: F821 # fmt: skip
    p: int  # noqa: F821 # fmt: skip


class add_point(NamedTuple):
    a: TKR[OpaqueType["numpy.ndarray"]]  # noqa: F821 # fmt: skip
    p: TKR[int]  # noqa: F821 # fmt: skip

    @staticmethod
    def out() -> type[TKR[PointedArray]]:  # noqa: F821 # fmt: skip
        return TKR[PointedArray]  # noqa: F821 # fmt: skip

    @property
    def namespace(self) -> str:
        return "scipy_worker"


class eval_point(NamedTuple):
    pa: TKR[PointedArray]  # noqa: F821 # fmt: skip

    @staticmethod
    def out() -> type[TKR[float]]:  # noqa: F821 # fmt: skip
        return TKR[float]  # noqa: F821 # fmt: skip

    @property
    def namespace(self) -> str:
        return "scipy_worker"


class linspace(NamedTuple):
    start: TKR[float]  # noqa: F821 # fmt: skip
    stop: TKR[float]  # noqa: F821 # fmt: skip
    num: TKR[int] | None = None  # noqa: F821 # fmt: skip

    @staticmethod
    def out() -> type[TKR[OpaqueType["numpy.ndarray"]]]:  # noqa: F821 # fmt: skip
        return TKR[OpaqueType["numpy.ndarray"]]  # noqa: F821 # fmt: skip

    @property
    def namespace(self) -> str:
        return "scipy_worker"


class transpose(NamedTuple):
    a: TKR[OpaqueType["numpy.ndarray"]]  # noqa: F821 # fmt: skip

    @staticmethod
    def out() -> type[TKR[OpaqueType["numpy.ndarray"]]]:  # noqa: F821 # fmt: skip
        return TKR[OpaqueType["numpy.ndarray"]]  # noqa: F821 # fmt: skip

    @property
    def namespace(self) -> str:
        return "scipy_worker"


class reshape(NamedTuple):
    a: TKR[OpaqueType["numpy.ndarray"]]  # noqa: F821 # fmt: skip
    shape: TKR[Union[int, list[int]]]  # noqa: F821 # fmt: skip

    @staticmethod
    def out() -> type[TKR[OpaqueType["numpy.ndarray"]]]:  # noqa: F821 # fmt: skip
        return TKR[OpaqueType["numpy.ndarray"]]  # noqa: F821 # fmt: skip

    @property
    def namespace(self) -> str:
        return "scipy_worker"
