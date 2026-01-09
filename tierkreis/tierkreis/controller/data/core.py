from dataclasses import dataclass
from typing import (
    Annotated,
    Any,
    Callable,
    Literal,
    NamedTuple,
    Protocol,
    SupportsIndex,
    get_args,
    get_origin,
    runtime_checkable,
)


SerializationFormat = Literal["bytes", "json", "unknown"]


class EmptyModel(NamedTuple): ...


@runtime_checkable
class RestrictedNamedTuple[T](Protocol):
    """A NamedTuple whose members are restricted to being of type T."""

    def _asdict(self) -> dict[str, T]: ...
    def __getitem__(self, key: SupportsIndex, /) -> T: ...


@dataclass
class Serializer:
    serializer: Callable[[Any], Any]
    serialization_method: SerializationFormat = "bytes"


@dataclass
class Deserializer:
    deserializer: Callable[[Any], Any]
    serialization_method: SerializationFormat = "bytes"


def get_t_from_args[T](t: type[T], hint: type | None) -> T | None:
    if hint is None or get_origin(hint) is not Annotated:
        return None

    for arg in get_args(hint):
        if isinstance(arg, t):
            return arg


def get_serializer(hint: type | None):
    return get_t_from_args(Serializer, hint)


def get_deserializer(hint: type | None):
    return get_t_from_args(Deserializer, hint)
