"""Code generated from TestNamespace namespace. Please do not edit."""

from typing import NamedTuple, Protocol

from tierkreis.controller.data.models import TKR
from tierkreis.controller.data.types import PType, Struct


class A(NamedTuple):
    age: TKR[int]  # fmt: skip
    name: TKR[dict[str, str]]  # fmt: skip


class B(Struct, Protocol):
    age: int  # fmt: skip
    name: dict[str, str]  # fmt: skip


class C[T: PType](Struct, Protocol):
    a: list[int]  # fmt: skip
    b: "B"  # fmt: skip
    included: "IncludedType"  # fmt: skip
    ol: "list[ListItem]"  # fmt: skip
    t: "T"  # fmt: skip


class IncludedType(Struct, Protocol):
    nested: "NestedType"  # fmt: skip


class ListItem(Struct, Protocol):
    i: int  # fmt: skip


class NestedType(Struct, Protocol):
    city: str  # fmt: skip


class foo(NamedTuple):
    a: TKR[int]  # fmt: skip
    b: TKR[str]  # fmt: skip

    @staticmethod
    def out() -> type[A]:  # fmt: skip
        return A  # fmt: skip

    @property
    def namespace(self) -> str:
        return "TestNamespace"


class bar(NamedTuple):
    @staticmethod
    def out() -> type[TKR[B]]:  # fmt: skip
        return TKR[B]  # fmt: skip

    @property
    def namespace(self) -> str:
        return "TestNamespace"


class z[T: PType](NamedTuple):
    c: TKR[C[T]]  # fmt: skip

    @staticmethod
    def out() -> type[TKR[C[T]]]:  # fmt: skip
        return TKR[C[T]]  # fmt: skip

    @property
    def namespace(self) -> str:
        return "TestNamespace"
