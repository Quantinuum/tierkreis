from collections.abc import Mapping, Sequence
from dataclasses import dataclass
from types import NoneType
from typing import Annotated, Self, get_args, get_origin

from tierkreis.controller.data.core import RestrictedNamedTuple
from tierkreis.controller.data.types import _is_generic

type ElementaryType = (
    type[int | float | bytes | str | bool | NoneType | Mapping | Sequence]
    | str  # Custom type e.g. forward reference
)


@dataclass
class GenericType:
    origin: ElementaryType
    args: "Sequence[GenericType | str]"

    @classmethod
    def from_type(cls, t: type) -> "Self":
        if get_origin(t) is Annotated:
            return cls.from_type(get_args(t)[0])

        args = get_args(t)
        origin = str(t) if _is_generic(t) else get_origin(t) or t

        subargs = []
        [subargs.append(str(x)) for x in args if _is_generic(x)]
        [subargs.append(cls.from_type(x)) for x in args if not _is_generic(x)]
        return cls(origin, subargs)

    @classmethod
    def _included_structs(cls, t: "GenericType") -> "set[GenericType]":
        outs = set({t}) if isinstance(t.origin, (RestrictedNamedTuple, str)) else set()
        [outs.update(cls._included_structs(x)) for x in t.args if isinstance(x, cls)]
        return outs

    def included_structs(self) -> "set[GenericType]":
        return GenericType._included_structs(self)

    def __hash__(self) -> int:
        return hash(self.origin)

    def __eq__(self, value: object) -> bool:
        if not hasattr(value, "origin"):
            return False
        return self.origin == value.origin


@dataclass
class TypedArg:
    name: str
    t: GenericType
    has_default: bool = False


@dataclass
class Method:
    name: GenericType
    args: list[TypedArg]
    return_type: GenericType
    return_type_is_portmapping: bool = False


@dataclass
class Interface:
    name: str
    methods: list[Method]


@dataclass
class Model:
    is_portmapping: bool
    t: GenericType
    decls: list[TypedArg]

    def __hash__(self) -> int:
        return hash(self.t.origin)

    def __lt__(self, other: "Model"):
        return str(self.t.origin) < str(other.t.origin)
