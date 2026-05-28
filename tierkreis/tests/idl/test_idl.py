from pathlib import Path

import pytest

import tests.idl.namespace1
from tierkreis.exceptions import TierkreisError
from tierkreis.idl.models import GenericType
from tierkreis.idl.type_symbols import type_symbol
from tierkreis.namespace import Namespace

type_symbols = [
    ("uint8", GenericType(int, [])),
    ("string", GenericType(str, [])),
    ("Array<integer>", GenericType(list, [GenericType(int, [])])),
    (
        "Record<Array<string>>",
        GenericType(
            dict,
            [GenericType(str, []), GenericType(list, [GenericType(str, [])])],
        ),
    ),
    (
        "Record<Array<C<T, A>>>",
        GenericType(
            dict,
            [
                GenericType(str, []),
                GenericType(list, [GenericType("C", ["T", "A"])]),
            ],
        ),
    ),
]
type_symbols_for_failure = ["decimal", "unknown", "duration"]
current_dir = Path(__file__).parent
typespecs = [(current_dir / "namespace1.tsp", tests.idl.namespace1.expected_namespace)]


@pytest.mark.parametrize(("type_symb", "expected"), type_symbols)
def test_type_t(type_symb: str, expected: type) -> None:
    assert (expected, "") == type_symbol(type_symb)


@pytest.mark.parametrize(("path", "expected"), typespecs)
def test_namespace(path: Path, expected: Namespace) -> None:
    namespace = Namespace.from_spec_file(path)
    assert namespace.stubs() == expected.stubs()

    # Write stubs to file.
    # This file will be subject to linting.
    # Also a change in this file can indicate an unexpectedly breaking change.
    namespace.write_stubs(Path(__file__).parent / "stubs_output.py")


@pytest.mark.parametrize("type_symb", type_symbols_for_failure)
def test_parser_fail(type_symb: str) -> None:
    with pytest.raises(TierkreisError):
        type_symbol(type_symb)


def test_model_order() -> None:
    namespace = tests.idl.namespace1.expected_namespace
    sorted_models = [
        str(model.t.origin.__qualname__)
        for model in namespace._topologically_sorted_models()
    ]
    assert sorted_models == [
        "B",
        "A",
        "NestedType",
        "IncludedType",
        "ListItem",
        "C",
    ]
