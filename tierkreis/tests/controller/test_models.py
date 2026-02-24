from types import NoneType
from typing import NamedTuple

import pytest

from tests.controller.test_types import ptypes
from tierkreis.controller.data.models import PModel, dict_from_pmodel, portmapping
from tierkreis.controller.data.types import PType


@portmapping
class NamedPModel(NamedTuple):
    a: bool
    b: int
    c: float
    d: str
    e: NoneType
    f: list[str]
    g: bytes
    h: tuple[int, str]
    i: list[list[int] | int]


@pytest.mark.parametrize("pmodel", ptypes)
def test_dict_from_pmodel_unnested(pmodel: PModel) -> None:
    assert dict_from_pmodel(pmodel) == {"value": pmodel}


named_p_model = NamedPModel(
    a=True,
    b=5,
    c=56.7,
    d="test",
    e=None,
    f=["one", "two", "three"],
    g=b"test bytes",
    h=(5, "test"),
    i=[[], 2],
)
named_p_model_expected = {
    "a": True,
    "b": 5,
    "c": 56.7,
    "d": "test",
    "e": None,
    "f": ["one", "two", "three"],
    "g": b"test bytes",
    "h": (5, "test"),
    "i": [[], 2],
}

pmodels = [(named_p_model, named_p_model_expected)]


@pytest.mark.parametrize(("pmodel", "expected"), pmodels)
def test_dict_from_pmodel_nested(pmodel: PModel, expected: dict[str, PType]) -> None:
    assert dict_from_pmodel(pmodel) == expected
