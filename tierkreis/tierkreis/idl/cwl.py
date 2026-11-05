"""Read CWL task interfaces for Python stub generation, not execution."""

# pyright: strict

import keyword
import re
from pathlib import Path
from typing import Protocol, cast

import yaml

from tierkreis.exceptions import TierkreisError
from tierkreis.idl.models import GenericType, Interface, Method, Model, TypedArg

_TYPES: dict[str, type] = {
    "boolean": bool,
    "int": int,
    "long": int,
    "float": float,
    "double": float,
    "string": str,
    "File": bytes,
}
_API_SYMBOLS = {
    "TKR",
    "NamedTuple",
    "NoneType",
    "OpaqueType",
    "PType",
    "Struct",
    "Workflow",
    "Literal",
    "Sequence",
    "TypeVar",
    "Generic",
    "Protocol",
    "Union",
    "int",
    "float",
    "bool",
    "str",
    "bytes",
    "list",
}


class _Loader(yaml.SafeLoader):
    pass


class _Constructor(Protocol):
    def construct_object(self, node: yaml.Node, deep: bool = False) -> object: ...


def _mapping(loader: yaml.SafeLoader, node: yaml.MappingNode) -> dict[str, object]:
    constructor = cast(_Constructor, loader)
    result: dict[str, object] = {}
    for key_node, value_node in node.value:
        key = constructor.construct_object(key_node)
        if not isinstance(key, str) or key in result:
            raise TierkreisError("CWL YAML keys must be unique strings")
        result[key] = constructor.construct_object(value_node)
    return result


_Loader.add_constructor("tag:yaml.org,2002:map", _mapping)


def _object(value: object, context: str) -> dict[str, object]:
    if not isinstance(value, dict):
        raise TierkreisError(f"{context}: expected an object")
    mapping = cast(dict[object, object], value)
    if any(not isinstance(key, str) for key in mapping):
        raise TierkreisError(f"{context}: expected string keys")
    return cast(dict[str, object], mapping)


def _fields(value: dict[str, object], allowed: set[str], context: str) -> None:
    unsupported = value.keys() - allowed - {"label", "doc"}
    if unsupported:
        raise TierkreisError(f"{context}: unsupported fields {sorted(unsupported)}")


def _identifier(value: object) -> str:
    if not isinstance(value, str):
        raise TierkreisError("CWL identifiers must be strings")
    name = value.removeprefix("#")
    if not re.fullmatch(r"[A-Za-z_][A-Za-z0-9_]*", name) or keyword.iskeyword(name):
        raise TierkreisError(f"Unsupported CWL identifier {value!r}")
    return name


def _entries(value: object, context: str) -> list[tuple[str, dict[str, object]]]:
    entries: list[tuple[str, dict[str, object]]] = []
    if isinstance(value, dict):
        for key, declaration in _object(cast(object, value), context).items():
            fields = (
                _object(cast(object, declaration), key)
                if isinstance(declaration, dict)
                else {"type": declaration}
            )
            entries.append((_identifier(key), fields))
    elif isinstance(value, list):
        for declaration in cast(list[object], value):
            fields = dict(_object(declaration, context))
            name = _identifier(fields.pop("id", None))
            entries.append((name, fields))
    else:
        raise TierkreisError(f"{context}: expected a map or list")
    names = [name for name, _ in entries]
    if len(set(names)) != len(names):
        raise TierkreisError(f"{context}: duplicate identifiers")
    return entries


def _type(value: object, context: str) -> GenericType:
    if isinstance(value, str) and value in _TYPES:
        return GenericType(_TYPES[value], [])
    if isinstance(value, str) and value.endswith("[]"):
        return GenericType(list, [_type(value[:-2], context)])
    if isinstance(value, dict):
        schema = _object(cast(object, value), context)
        _fields(schema, {"type", "items"}, context)
        if schema.get("type") == "array":
            return GenericType(list, [_type(schema.get("items"), context)])
    raise TierkreisError(f"{context}: unsupported CWL type {value!r}")


def _port(name: str, fields: dict[str, object], *, output: bool) -> TypedArg:
    if name.startswith("_") or (not output and name in {"namespace", "out"}):
        raise TierkreisError(
            f"{name}: port name conflicts with the generated Python API"
        )
    binding = "outputBinding" if output else "inputBinding"
    _fields(fields, {"type", binding}, name)
    if binding in fields:
        _object(fields[binding], binding)
    return TypedArg(name, _type(fields.get("type"), name))


def parse_file(path: Path) -> tuple[list[Model], Interface]:
    """Extract the typed interface of one CWL v1.2 CommandLineTool.

    Command fields and bindings are descriptive only. Workers must implement
    the Tierkreis WorkerCallArgs protocol themselves.
    """
    try:
        document = _object(yaml.load(path.read_text(), Loader=_Loader), str(path))
        _fields(
            document,
            {
                "cwlVersion",
                "class",
                "id",
                "baseCommand",
                "arguments",
                "stdin",
                "stdout",
                "stderr",
                "inputs",
                "outputs",
            },
            str(path),
        )
        if (
            document.get("cwlVersion") != "v1.2"
            or document.get("class") != "CommandLineTool"
        ):
            raise TierkreisError(
                "Only CWL v1.2 CommandLineTool interfaces are supported"
            )
        name = _identifier(document.get("id", path.stem))
        if name in _API_SYMBOLS:
            raise TierkreisError(
                f"{name}: task name conflicts with the generated Python API"
            )
        inputs = [
            _port(n, f, output=False)
            for n, f in _entries(document.get("inputs"), "inputs")
        ]
        outputs = [
            _port(n, f, output=True)
            for n, f in _entries(document.get("outputs"), "outputs")
        ]
        model = Model(True, GenericType(f"{name}_outputs", []), outputs)
        method = Method(GenericType(name, []), inputs, model.t, True)
        return [model], Interface(path.stem, [method])
    except (ValueError, yaml.YAMLError, TierkreisError) as error:
        raise TierkreisError(f"{path}: {error}") from error
