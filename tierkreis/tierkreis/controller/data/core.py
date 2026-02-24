"""Core types in tierkreis.

- PortID = str, name of an output on a node
- NodeIndex = int, index of node in the graph list
- ValueRef = tuple[NodeIndex, PortID] reference of a value;
    uniquely identified by the node and its output.
"""

from collections.abc import Callable
from dataclasses import dataclass
from typing import (
    Annotated,
    Any,
    Literal,
    NamedTuple,
    Protocol,
    SupportsIndex,
    get_args,
    get_origin,
    runtime_checkable,
)

PortID = str
NodeIndex = int
ValueRef = tuple[NodeIndex, PortID]
SerializationFormat = Literal["bytes", "json", "unknown"]


class EmptyModel(NamedTuple):
    """A model without content."""


@runtime_checkable
class RestrictedNamedTuple[T](Protocol):
    """A NamedTuple whose members are restricted to being of type T."""

    def _asdict(self) -> dict[str, T]: ...
    def __getitem__(self, key: SupportsIndex, /) -> T:
        """Access the indexed element as in a tuple."""
        ...


@dataclass
class Serializer:
    """Serializer for tkr values.

    :fields:
        serializer (Callable[[Any], Any]): A function taking a value producing a
            serialized version of it.
        serialization_method (Literal): Indicator of serializer type of
            ["bytes", "json", "unknown"], defaults to "bytes".
    """

    serializer: Callable[[Any], Any]
    serialization_method: SerializationFormat = "bytes"


@dataclass
class Deserializer:
    """Serializer for tkr values.

    :fields:
        serializer (Callable[[Any], Any]): A function taking a serialized
            value producing a deserialized value.
        serialization_method (Literal): Indicator of serializer type of
            ["bytes", "json", "unknown"], defaults to "bytes".
    """

    deserializer: Callable[[Any], Any]
    serialization_method: SerializationFormat = "bytes"


def get_t_from_args[T](t: type[T], hint: type | None) -> T | None:
    """Get the possible type generic T from a type.

    :return: The generic hint T if it exists on the value.
        Either from its annotation or type hint.
    :rtype: T | None
    """
    if hint is None or get_origin(hint) is not Annotated:
        return None

    for arg in get_args(hint):
        if isinstance(arg, t):
            return arg
    return None


def get_serializer(hint: type | None) -> Serializer | None:
    """Get the serializer for an annotated type.

    This is relevant for annotated worker types
    AnnotatedType = Annotated[BaseType, ser, deser]
    worker_fn: AnnotatedType -> AnnotatedType

    :param hint: The type to get the serializer for.
    :type hint: type | None
    :return: The deserializer if one is annotated.
    :rtype: Deserializer | None
    """
    return get_t_from_args(Serializer, hint)


def get_deserializer(hint: type | None) -> Deserializer | None:
    """Get the deserializer for an annotated type.

    This is relevant for annotated worker types
    AnnotatedType = Annotated[BaseType, ser, deser]
    worker_fn: AnnotatedType -> AnnotatedType

    :param hint: The type to get the deserializer for.
    :type hint: type | None
    :return: The deserializer if one is annotated.
    :rtype: Deserializer | None
    """
    return get_t_from_args(Deserializer, hint)
