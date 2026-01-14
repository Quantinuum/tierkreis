from types import NoneType

import pytest

from tierkreis.codegen import format_generic_type
from tierkreis.controller.data.types import PType
from tierkreis.idl.models import GenericType

formats: list[tuple[type[PType], str]] = [
    (bool, "bool"),
    (int, "int"),
    (float, "float"),
    (str, "str"),
    (bytes, "bytes"),
    (NoneType, "NoneType"),
    (list[str], "list[str]"),
    (list[str | list[str | int]], "list[Union[str, list[Union[str, int]]]]"),
    (tuple[str], "tuple[str]"),
    (
        tuple[str | list[str | int], NoneType],
        "tuple[Union[str, list[Union[str, int]]], NoneType]",
    ),
]


@pytest.mark.parametrize(("ttype", "expected"), formats)
def test_format_ttype(ttype: type[PType], expected: str) -> None:
    generic_type = GenericType.from_type(ttype)

    assert (
        format_generic_type(generic_type, include_bound=False, is_tkr=False) == expected
    )
