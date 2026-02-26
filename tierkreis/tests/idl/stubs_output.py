"""Code generated from TestNamespace namespace. Please do not edit."""

from typing import NamedTuple, Protocol
from tierkreis.controller.data.models import TKR
from tierkreis.controller.data.types import PType, Struct


class A(NamedTuple):
    age: TKR[int]  # noqa: F821 # fmt: skip
    name: TKR[dict[str, str]]  # noqa: F821 # fmt: skip


class B(Struct, Protocol):
    age: int  # noqa: F821 # fmt: skip
    name: dict[str, str]  # noqa: F821 # fmt: skip


class C[T: PType](Struct, Protocol):
    a: list[int]  # noqa: F821 # fmt: skip
    b: "B"  # noqa: F821 # fmt: skip
    included: "IncludedType"  # noqa: F821 # fmt: skip
    ol: "list[ListItem]"  # noqa: F821 # fmt: skip
    t: "T"  # noqa: F821 # fmt: skip


class IncludedType(Struct, Protocol):
    nested: "NestedType"  # noqa: F821 # fmt: skip


class ListItem(Struct, Protocol):
    i: int  # noqa: F821 # fmt: skip


class NestedType(Struct, Protocol):
    city: str  # noqa: F821 # fmt: skip


class foo(NamedTuple):
    a: TKR[int]  # noqa: F821 # fmt: skip
    b: TKR[str]  # noqa: F821 # fmt: skip

    @staticmethod
    def out() -> type[A]:  # noqa: F821 # fmt: skip
        return A  # noqa: F821 # fmt: skip

    @property
    def namespace(self) -> str:
        return "TestNamespace"


class bar(NamedTuple):
    @staticmethod
    def out() -> type[TKR[B]]:  # noqa: F821 # fmt: skip
        return TKR[B]  # noqa: F821 # fmt: skip

    @property
    def namespace(self) -> str:
        return "TestNamespace"


class z[T: PType](NamedTuple):
    c: TKR[C[T]]  # noqa: F821 # fmt: skip

    @staticmethod
    def out() -> type[TKR[C[T]]]:  # noqa: F821 # fmt: skip
        return TKR[C[T]]  # noqa: F821 # fmt: skip

    @property
    def namespace(self) -> str:
        return "TestNamespace"
