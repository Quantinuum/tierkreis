from typing import NamedTuple

from tierkreis.data.models import portmapping
from tierkreis.data.types import PType
from tierkreis.worker import Worker

worker = Worker("TestNamespace")


class ListItem(NamedTuple):
    i: int


class NestedType(NamedTuple):
    city: str


class IncludedType(NamedTuple):
    nested: NestedType


@portmapping
class A(NamedTuple):
    name: dict[str, str]
    age: int
    b: "B"
    bs: list[list["B"]]


class B(NamedTuple):
    name: dict[str, str]
    age: int


class C[T: PType](NamedTuple):
    a: list[int]
    b: B
    t: T
    included: IncludedType
    ol: list[ListItem]


@worker.task()
def foo(a: int, b: str) -> A: ...
@worker.task()
def bar() -> B: ...
@worker.task()
def z[T: PType](c: C[T]) -> C[T]: ...


expected_namespace = worker.namespace
