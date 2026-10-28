"""Tests for the Rust builtin worker's Python API."""

from tierkreis._tierkreis import (  # pyright: ignore[reportMissingImports]
    builtin_task_names,
    execute_builtin,
)

import tierkreis.builtins as builtins  # noqa: PLR0402


def test_builtin_exports_match_rust_registry() -> None:
    """Every Rust builtin has a typed Python graph-building stub."""
    exported_tasks = set(builtins.__all__) - {"Headed", "Untupled", "Unzipped"}
    assert exported_tasks == set(builtin_task_names())


def test_legacy_names_construct_the_named_task() -> None:
    """Aliases retain the task name used by the worker protocol."""
    assert builtins.imul.__name__ == "imul"
    assert builtins.str.__name__ == "str"


def test_execute_builtin_uses_serialized_values() -> None:
    """The Python in-memory adapter invokes the shared Rust dispatcher."""
    assert execute_builtin("iadd", {"a": b"2", "b": b"3"}) == {"value": b"5"}
