import subprocess
import sys
from collections.abc import Mapping, Sequence
from dataclasses import dataclass
from datetime import datetime
from types import NoneType, UnionType
from uuid import UUID

import pytest
from pydantic import BaseModel

from tests.controller.typed_graphdata import typed_doubler
from tierkreis.data.graph import GraphData
from tierkreis.data.models import TKR
from tierkreis.data.types import (
    PType,
    Workflow,
    bytes_from_ptype,
    is_ptype,
    ptype_from_bytes,
)


class UntupledModel[U, V](BaseModel):
    a: U
    b: V


class DummyBaseModel(BaseModel):
    a: int
    b: bytes


@dataclass
class DummyDictConvertible:
    a: int

    def to_dict(self) -> dict:
        return {"a": self.a}

    @classmethod
    def from_dict(cls, args: dict) -> "DummyDictConvertible":
        return DummyDictConvertible(a=args["a"])


@dataclass
class DummyListConvertible:
    a: int

    def to_list(self) -> list:
        return list(range(self.a))

    @classmethod
    def from_list(cls, args: list) -> "DummyListConvertible":
        return DummyListConvertible(a=len(args))


type_list: Sequence[type[PType] | UnionType] = []
type_list.append(bool)
type_list.append(int)
type_list.append(str)
type_list.append(float)
type_list.append(NoneType)
type_list.append(bytes)
type_list.append(Sequence[str])
type_list.append(Sequence[Sequence[Sequence[NoneType]]])
type_list.append(Sequence[str | Sequence[int]])
type_list.append(Sequence[str | Sequence[int | float]])
type_list.append(int | None)
type_list.append(int | bytes)
type_list.append(tuple[int, str])
type_list.append(tuple[int | str])
type_list.append(Mapping[str, dict[str, bytes]])
type_list.append(GraphData)
type_list.append(Workflow[TKR[int], TKR[bool]])

fail_list: Sequence[type] = []
fail_list.append(UUID)
fail_list.append(datetime)
fail_list.append(list[datetime])
fail_list.append(Mapping[str, dict[str, datetime]])

ptypes: Sequence[PType] = [
    True,
    False,
    0,
    1,
    2,
    "0",
    "1",
    "test",
    0.0,
    0.1,
    15.2,
    None,
    ["one", 1],
    [1, 2, 3],
    [[[]], 1, ["45", "test", b"test bytes"]],
    b"these are some test bytes",
    {"one": 1, "two": "two", "bytes": b"asdf"},
    DummyDictConvertible(a=11),
    DummyListConvertible(a=5),
    DummyBaseModel(a=5, b=b"some bytes"),
    complex(1, -1),
]


@pytest.mark.parametrize("ptype", ptypes)
def test_bytes_roundtrip(ptype: PType) -> None:
    bs = bytes_from_ptype(ptype)
    new_type = ptype_from_bytes(bs, type(ptype))
    assert ptype == new_type


annotated_ptypes: Sequence[tuple[PType, type]] = [
    (ptype, type(ptype)) for ptype in ptypes
] + [  # Not possible to deserialise without annotations
    (typed_doubler(), Workflow[TKR[int], TKR[int]]),
    (
        [DummyListConvertible(a=1), DummyListConvertible(a=2)],
        list[DummyListConvertible],
    ),
    (
        {"a": DummyListConvertible(a=1), "b": DummyListConvertible(a=2)},
        dict[str, DummyListConvertible],
    ),
]


@pytest.mark.parametrize("annotated_ptype", annotated_ptypes)
def test_annotated_bytes_roundtrip(annotated_ptype: tuple[PType, type]) -> None:
    ptype, annotation = annotated_ptype
    bs = bytes_from_ptype(ptype)
    new_type = ptype_from_bytes(bs, annotation)
    assert ptype == new_type


@pytest.mark.parametrize("ptype", type_list)
def test_ptype_from_annotation(ptype: type[PType]) -> None:
    assert is_ptype(ptype)


@pytest.mark.parametrize("ptype", fail_list)
def test_ptype_from_annotation_fails(ptype: type[PType]) -> None:
    assert not is_ptype(ptype)


def test_scalar_worker_does_not_import_hugr() -> None:
    code = """
import sys
from tierkreis.data.types import bytes_from_ptype, ptype_from_bytes
from tierkreis.worker import Worker

worker = Worker("scalar")

@worker.task()
def add(a: int, b: int) -> int:
    return a + b

encoded = bytes_from_ptype(42, int)
assert ptype_from_bytes(encoded, int) == 42
assert "hugr" not in sys.modules
assert "hugr.package" not in sys.modules
"""
    subprocess.run([sys.executable, "-c", code], check=True)


def test_hugr_package_type_and_roundtrip() -> None:
    from hugr.package import Package

    class DerivedPackage(Package):
        pass

    package = Package([])
    assert is_ptype(Package)
    assert is_ptype(DerivedPackage)
    assert ptype_from_bytes(bytes_from_ptype(package, Package), Package) == package
