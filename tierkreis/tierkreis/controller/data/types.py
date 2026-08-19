"""Valid Python types for annotating worker functions and their serialisation."""

import json
import logging
import pickle
from base64 import b64decode, b64encode
from collections import defaultdict
from collections.abc import Mapping, Sequence
from dataclasses import dataclass
from inspect import Parameter, _empty, isclass
from types import NoneType, UnionType
from typing import (
    TYPE_CHECKING,
    Annotated,
    Any,
    Protocol,
    Self,
    TypeVar,
    Union,
    assert_never,
    cast,
    get_args,
    get_origin,
    overload,
    runtime_checkable,
)

from hugr.package import Package
from pydantic import BaseModel, ValidationError
from typing_extensions import TypeIs

from tierkreis.controller.data.core import (
    Deserializer,
    RestrictedNamedTuple,
    SerializationFormat,
    Serializer,
    get_deserializer,
    get_serializer,
)
from tierkreis.exceptions import TierkreisError

if TYPE_CHECKING:
    from tierkreis.controller.data.graph import GraphData
    from tierkreis.controller.data.models import TModel

logger = logging.getLogger(__name__)


@runtime_checkable
class NdarraySurrogate(Protocol):
    """A protocol to enable use of numpy.ndarray.

    By default the serialisation will be done using dumps
    and the deserialisation using `pickle.loads`.

    The semantics are left to the implementor.
    """

    def dumps(self) -> bytes:
        """Dump self to bytes."""
        ...

    def tobytes(self) -> bytes:
        """Transform self to bytes."""
        ...

    def tolist(self) -> list:
        """Convert self to a list."""
        ...


@runtime_checkable
class DictConvertible(Protocol):
    """A protocol for types that can be converted to and from dicts."""

    def to_dict(self) -> dict:
        """Convert self to a dict."""
        ...

    @classmethod
    def from_dict(cls, arg: dict, /) -> "Self":
        """Construct self from a dict."""
        ...


@runtime_checkable
class ListConvertible(Protocol):
    """A protocol for types that can be converted to and from lists."""

    def to_list(self) -> list:
        """Convert self to a list."""
        ...

    @classmethod
    def from_list(cls, arg: list, /) -> "Self":
        """Construct self from a list."""
        ...


type Container[T] = (
    T
    | list[Container[T]]
    | Sequence[Container[T]]
    | tuple[Container[T], ...]
    | dict[str, Container[T]]
    | Mapping[str, Container[T]]
)
type ElementaryType = (
    bool
    | int
    | float
    | complex
    | str
    | NoneType
    | bytes
    | DictConvertible
    | ListConvertible
    | NdarraySurrogate
    | BaseModel  # Includes GraphData
    | Workflow  # So, special case: a Workflow is just a GraphData, discard the type info
    | Package
)
type JsonType = Container[ElementaryType]


@runtime_checkable
class Struct(RestrictedNamedTuple[JsonType], Protocol):
    """Supertype for structs, which are named tuples with JSON-serialisable fields."""


type _StructPType = JsonType | Struct
PType = Container[_StructPType]
"""A restricted subset of Python types that can be used to annotate
worker functions for automatic codegen of graph builder stubs."""


class TierkreisEncoder(json.JSONEncoder):
    """Encode bytes also."""

    def default(self, o) -> dict[str, Any] | dict[str, list[float]] | Any:
        """Call the default tierkreis serializer.

        :param o: The object to serialize.
        :type o: _type_
        :return: The serialized object.
        :rtype: dict[str, Any] | dict[str, list[float]] | Any
        """
        if isinstance(o, bytes):
            return {"__tkr_bytes__": True, "bytes": b64encode(o).decode()}

        if isinstance(o, complex):
            return [o.real, o.imag]

        return super().default(o)


class TierkreisDecoder(json.JSONDecoder):
    """Decode bytes also."""

    def __init__(self, **kwargs) -> None:
        kwargs.setdefault("object_hook", self._object_hook)
        super().__init__(**kwargs)

    def _object_hook(self, d) -> bytes | complex | Any:
        """Try to decode an object containing bytes."""
        if "__tkr_bytes__" in d and "bytes" in d:
            return b64decode(d["bytes"])

        return d


def _is_union(o: object) -> bool:
    return (
        o in (Union, UnionType) or get_origin(o) == UnionType or get_origin(o) == Union
    )


def is_optional(t: type) -> bool:
    """Check that the origin of `t` is a Union and the args of `t` contains NoneType."""
    origin = get_origin(t)
    if origin is None:
        return False

    is_union = origin == Union or (isclass(origin) and issubclass(origin, UnionType))
    return bool(is_union and NoneType in get_args(t))


def _is_generic(o) -> TypeIs[type[TypeVar]]:
    return isinstance(o, TypeVar)


def _is_list(ptype: object) -> TypeIs[type[Sequence[PType]]]:
    return get_origin(ptype) == Sequence or get_origin(ptype) is list


def _is_mapping(ptype: object) -> TypeIs[type[Mapping[str, PType]]]:
    return get_origin(ptype) is Mapping or get_origin(ptype) is dict


def _is_tuple(o: object) -> TypeIs[type[tuple[Any, ...]]]:
    return get_origin(o) is tuple


def is_ptype(annotation: Any) -> TypeIs[type[PType]]:
    """Check if a type annotation is a PType.

    :param annotation: The annotation to check.
    :type annotation: Any
    :return: The according TypeIs if the annotation is a PType, otherwise False.
    :rtype: TypeIs[type[PType]]
    """
    origin = get_origin(annotation)
    if origin is Annotated:
        args = get_args(annotation)
        if (
            len(args) == 3
            and isinstance(args[1], Serializer)
            and isinstance(args[2], Deserializer)
        ):
            # User specified serialized types are treated as ptype
            return True

        return is_ptype(args[0])

    if _is_generic(annotation):
        return True

    if (
        _is_union(annotation)
        or _is_tuple(annotation)
        or _is_list(annotation)
        or _is_mapping(annotation)
    ):
        return all(is_ptype(x) for x in get_args(annotation))

    if (
        isclass(annotation)
        and issubclass(
            annotation,
            (DictConvertible, ListConvertible, NdarraySurrogate, BaseModel, Struct),
        )
        or (isclass(origin) and issubclass(origin, Workflow))
        or annotation in get_args(ElementaryType.__value__)
    ):
        return True

    origin = get_origin(annotation)
    if origin is not None:
        return is_ptype(origin) and all(is_ptype(x) for x in get_args(annotation))

    return False


def ser_from_ptype(ptype: PType, annotation: type[PType] | None) -> JsonType:
    """Get the json serializable type of a ptype value.

    Potentially uses a custom serializer if the annotation has one.

    :param ptype: The type to serialize.
    :type ptype: PType
    :param annotation: The annotation of the ptype, if available.
    :type annotation: type[PType] | None
    :return: The serialized ptype.
    :rtype: JsonType
    """
    if sr := get_serializer(annotation):
        return sr.serializer(ptype)

    match ptype:
        case Workflow():
            return ser_from_ptype(ptype.data, annotation)
        case bytes() | bytearray() | memoryview():
            return bytes(ptype)
        case Package():
            return ptype.to_bytes()
        case bool() | int() | float() | complex() | str() | NoneType() | TypeVar():
            return ptype
        case Struct():
            d = annotation.__annotations__ or defaultdict(None)
            return {k: ser_from_ptype(p, d[k]) for k, p in ptype._asdict().items()}
        case tuple():
            args = get_args(annotation) or [None] * len(ptype)
            return tuple([ser_from_ptype(p, args[i]) for i, p in enumerate(ptype)])
        case Sequence():
            arg = get_args(annotation)[0] if get_args(annotation) else None
            return [ser_from_ptype(p, arg) for p in ptype]
        case Mapping():
            arg = get_args(annotation)[1] if get_args(annotation) else None
            return {k: ser_from_ptype(p, arg) for k, p in ptype.items()}
        case DictConvertible():
            return ser_from_ptype(ptype.to_dict(), None)
        case ListConvertible():
            return ser_from_ptype(ptype.to_list(), None)
        case BaseModel():
            return ptype.model_dump(mode="json")
        case NdarraySurrogate():
            return ptype.dumps()
        case _:
            assert_never(ptype)


def bytes_from_ptype(ptype: PType, annotation: type[PType] | None = None) -> bytes:
    """Get the bytes representation of a ptype value.

    :param ptype: The ptype value to convert to bytes.
    :type ptype: PType
    :param annotation: The annotation of the ptype, if available.
    :type annotation: type[PType] | None, optional
    :return: The bytes representation of the ptype value.
    :rtype: bytes
    """
    ser = ser_from_ptype(ptype, annotation)
    match ser:
        case bytes():
            return ser  # Top level bytes should be a clean pass-through.
        case _:
            return json.dumps(ser, cls=TierkreisEncoder, indent=2).encode()


def coerce_from_annotation[T: PType](ser: Any, annotation: type[T] | None) -> T | None:
    """Find the value of type T from a serialized form.

    Uses the annotation to find the correct deserialization method, if available.

    :param ser: The value to coerce.
    :type ser: Any
    :param annotation: The annotation to coerce to, if available.
    :type annotation: type[T] | None, optional
    :raises TierkreisError: If the value cannot be coerced to the annotation.
    :return: The coerced value or None if the input was None.
    :rtype: T | None
    """
    from tierkreis.controller.data.graph import GraphData

    if annotation is None:
        return ser

    if ds := get_deserializer(annotation):
        return ds.deserializer(ser)

    origin = get_origin(annotation)
    if origin is Annotated:
        return coerce_from_annotation(ser, get_args(annotation)[0])

    if _is_union(annotation):
        for t in get_args(annotation):
            try:
                return coerce_from_annotation(ser, t)
            except (AssertionError, ValidationError):
                logger.debug("Tried deserialising as %s", t)
        msg = f"Could not deserialise {ser} as {annotation}"
        raise TierkreisError(msg)

    if origin is None:
        origin = annotation

    if ser is None:
        return ser

    if isinstance(origin, TypeVar):
        # Required to support generic parameters in functions,
        # we can't really make a judgement about what type it
        # should be deserialised in this case and so have to
        # just return the value in its "raw" form.
        return ser

    if issubclass(origin, (bool, int, float, str, bytes, NoneType)):
        return ser

    if issubclass(origin, complex):
        return complex(real=ser[0], imag=ser[1])  # type: ignore complex is T

    if origin is Package:
        return Package.from_bytes(ser)  # type: ignore Package is T

    if issubclass(origin, DictConvertible):
        if not issubclass(annotation, origin):
            msg = "Invalid subclass relation encountered."
            raise TypeError(msg)
        return annotation.from_dict(ser)

    if issubclass(origin, ListConvertible):
        if not issubclass(annotation, origin):
            msg = "Invalid subclass relation encountered."
            raise TypeError(msg)
        return annotation.from_list(ser)

    if issubclass(origin, NdarraySurrogate):
        return pickle.loads(ser)

    if issubclass(origin, BaseModel):
        if not issubclass(annotation, origin):
            msg = "Invalid subclass relation encountered."
            raise TypeError(msg)
        return annotation(**ser)

    if issubclass(origin, Workflow):
        _inputs, outputs = get_args(annotation)
        return annotation(coerce_from_annotation(ser, GraphData), outputs)  # type: ignore

    if issubclass(origin, Struct):
        if isinstance(ser, Sequence):
            xs = [
                coerce_from_annotation(x, annotation)
                for (x, annotation) in zip(ser, origin.__annotations__.values())
            ]
            return cast("T", origin(*xs))
        d = {
            k: coerce_from_annotation(ser[k], v)
            for k, v in origin.__annotations__.items()
        }
        return cast("T", origin(**d))

    if issubclass(origin, Sequence):
        args = get_args(annotation)
        if len(args) == 0:
            return ser

        return cast("T", [coerce_from_annotation(x, args[0]) for x in ser])

    if issubclass(origin, Mapping):
        args = get_args(annotation)
        if len(args) == 0:
            return ser

        return cast(
            "T",
            {k: coerce_from_annotation(v, args[1]) for k, v in ser.items()},
        )

    assert_never(ser)


def get_serialization_format[T: PType](
    hint: type[T] | None = None,
) -> SerializationFormat:
    """Find the serializaiton format to a type hint.

    Returns 'unknown' for None.

    :param hint: The type hint to find the serialization format for.
    :type hint: type[T] | None, optional
    :return: The serialization format for the given type hint.
    :rtype: SerializationFormat
    """
    if hint is None:
        return "unknown"

    if sr := get_serializer(hint):
        return sr.serialization_method

    unannotated = get_args(hint)[0] if get_origin(hint) is Annotated else hint
    if (
        isclass(unannotated)
        and issubclass(unannotated, (bytes, NdarraySurrogate))
        or isclass(unannotated)
        and issubclass(unannotated, Package)
    ):
        return "bytes"

    return "json"


@overload
def ptype_from_bytes[T: PType](bs: bytes, annotation: type[T] | None = None) -> T: ...
@overload
def ptype_from_bytes[T: PType](bs: None, annotation: type[T] | None = None) -> None: ...
def ptype_from_bytes[T: PType](
    bs: bytes | None, annotation: type[T] | None = None
) -> T | None:
    """Get the value with the correct type from its bytes.

    :param bs: The bytes to deserialize.
    :type bs: bytes | None
    :param annotation: The annotation to use for deserialization, if available.
    :type annotation: type[T] | None, optional
    :return: The deserialized value of type T, or None if bytes was None.
    :rtype: T | None
    """
    if bs is None:
        return None
    method = get_serialization_format(annotation)
    match method:
        case "bytes":
            if (result := coerce_from_annotation(bs, annotation)) is not None:
                return result
            return None
        case "json":
            j = json.loads(bs, cls=TierkreisDecoder)
            if (result := coerce_from_annotation(j, annotation)) is not None:
                return result
            return None
        case "unknown":
            try:
                j = json.loads(bs, cls=TierkreisDecoder)
                if (result := coerce_from_annotation(j, annotation)) is not None:
                    return result
                return None
            except (json.JSONDecodeError, UnicodeDecodeError):
                return cast("T", bs)
        case _:
            assert_never(method)


def has_default(t: Parameter) -> bool:
    """Check if a parameter has a default value.

    :param t: The parameter to check.
    :type t: Parameter
    :return: True if the parameter has a default value, False otherwise.
    :rtype: bool
    """
    return not (isclass(t.default) and issubclass(t.default, _empty))


@dataclass(frozen=True)
class Workflow[Inputs: TModel, Outputs: TModel]:
    data: "GraphData"
    outputs_type: type[Outputs]
