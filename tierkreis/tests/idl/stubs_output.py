"""Code generated from TestNamespace namespace. Please do not edit."""

from typing import NamedTuple, Protocol

from tierkreis.controller.data.models import TKR
from tierkreis.controller.data.types import PType, Struct


class B(Struct, Protocol):
    age: int
    name: dict[str, str]


class A(NamedTuple):
    age: TKR[int]
    b: TKR[B]
    bs: TKR[list[list[B]]]
    name: TKR[dict[str, str]]


class NestedType(Struct, Protocol):
    city: str


class IncludedType(Struct, Protocol):
    nested: NestedType


class ListItem(Struct, Protocol):
    i: int


class C[T: PType](Struct, Protocol):
    a: list[int]
    b: B
    included: IncludedType
    ol: list[ListItem]
    t: T


class foo(NamedTuple):
    a: TKR[int]
    b: TKR[str]

    @staticmethod
    def out() -> type[A]:
        return A

    @property
    def namespace(self) -> str:
        return "TestNamespace"


class bar(NamedTuple):
    @staticmethod
    def out() -> type[TKR[B]]:
        return TKR[B]

    @property
    def namespace(self) -> str:
        return "TestNamespace"


class z[T: PType](NamedTuple):
    c: TKR[C[T]]

    @staticmethod
    def out() -> type[TKR[C[T]]]:
        return TKR[C[T]]

    @property
    def namespace(self) -> str:
        return "TestNamespace"
