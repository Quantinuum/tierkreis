"""Tierkreis IDL models representation used for TSP parsing."""

from collections.abc import Mapping, Sequence
from dataclasses import dataclass
from types import NoneType
from typing import Annotated, Self, get_args, get_origin

from tierkreis.controller.data.core import (
    Deserializer,
    RestrictedNamedTuple,
    Serializer,
)
from tierkreis.controller.data.types import _is_generic, is_ptype

type ElementaryType = (
    type[int | float | bytes | str | bool | NoneType | Mapping | Sequence]
    | str  # Custom type e.g. forward reference
)


@dataclass
class GenericType:
    """A Tierkreis worker generic type.

    Represents a single (composed) type in worker definitions.

    :fields:
        origin (ElementaryType): The base type, e.g., str in list[str].
        args:  (Sequence[GenericType | str]) The nested types.
            e.g., list[str] in set[list[str]]
        is_ptype (bool): true if the type was either
            (a) constructed from a `type` that was a PType,
            (b) not constructed from a `type` at all (e.g. from a string in the parser).
        has_serialization (bool): true if the type has custom serialization.
        is_forward_ref (bool): true if the type is a forward reference e.g. list["MyType"] or "MyType".

    """

    origin: ElementaryType
    args: "Sequence[GenericType | str]"
    is_ptype: bool = True
    has_serialization: bool = False
    is_forward_ref: bool = False

    @classmethod
    def from_type(cls, t: type | str, *, has_serialization: bool = False) -> "Self":
        """Construct a generic type from a python type.

        :param t: The python type.
        :type t: type
        :param has_serialization: If the type has custom serialization.
        :type has_serialization: bool, defaults to False.
        :return: The Tierkreis type.
        :rtype: Self
        """
        if isinstance(t, str):
            return cls(t, [], is_forward_ref=True)
        if get_origin(t) is Annotated:
            args = get_args(t)
            if (
                len(args) == 3
                and isinstance(args[1], Serializer)
                and isinstance(args[2], Deserializer)
            ):
                return cls.from_type(args[0], has_serialization=True)
            return cls.from_type(get_args(t)[0])

        args = get_args(t)
        origin = str(t) if _is_generic(t) else get_origin(t) or t

        subargs = []
        [subargs.append(str(x)) for x in args if _is_generic(x)]
        [subargs.append(cls.from_type(x)) for x in args if not _is_generic(x)]
        return cls(origin, subargs, is_ptype(t), has_serialization)

    @classmethod
    def _included_structs(cls, t: "GenericType") -> "set[GenericType]":
        outs = set({t}) if isinstance(t.origin, (RestrictedNamedTuple, str)) else set()
        [outs.update(cls._included_structs(x)) for x in t.args if isinstance(x, cls)]
        return outs

    def included_structs(self) -> "set[GenericType]":
        """Find the included structs of this type.

        A struct is an instance of RestrictedNamedTuple or opaque strings.
        :return: The list of structs
        :rtype: set[GenericType]
        """
        return GenericType._included_structs(self)

    def __hash__(self) -> int:
        """Produce a hash of the generic type.

        :return: The hash.
        :rtype: int
        """
        return hash(self.origin)

    def __eq__(self, value: object) -> bool:
        """Check the equality of self with an object.

        self == value <==> self.origin == value.origin


        :param other: The object  to compare to.
        :type other: object
        :return: If both object have the same origin.
        :rtype: bool
        """
        if not hasattr(value, "origin"):
            return False
        return self.origin == getattr(value, "origin")


@dataclass
class TypedArg:
    """A Tierkreis worker method argument.

    Represents a single argument to a tasks in a worker.
    :fields:
        name (str): The argument name.
        t (GenericType): The argument type.
        has_default(bool): Whether the argument has a default value.
    """

    name: str
    t: GenericType
    has_default: bool = False


@dataclass
class Method:
    """A Tierkreis worker method.

    Represents a tasks in a worker.

    :fields:
        name (str): The method name.
        args (list[TypedArg]): The list of method arguments.
        return_type (GenericType): The method return type.
        return_type_is_portmapping (bool): Whether the return_type is a portmapping.
    """

    name: GenericType
    args: list[TypedArg]
    return_type: GenericType
    return_type_is_portmapping: bool = False


@dataclass
class Interface:
    """A Tierkreis worker interface.

    Represents a list of tasks contained in the worker.

    :fields:
        name (str): The worker name.
        methods (list[Method]): The available tasks in the worker.
    """

    name: str
    methods: list[Method]


@dataclass
class Model:
    """A Tierkreis worker model.

    Represents a type in a worker.

    :fields:
        is_portmapping (bool): Whether the model is a portmapping.
        t (GenericType): The type of the model.
        decl (list[TypedArg]) The list of its typed arguments.
    """

    is_portmapping: bool
    t: GenericType
    decls: list[TypedArg]

    def __hash__(self) -> int:
        """Produce a hash of the model.

        :return: The hash.
        :rtype: int
        """
        return hash(self.t.origin)

    def __lt__(self, other: "Model") -> bool:
        """Check order of two models.

        Uses lexicographical ordering of the origin of the models (generic) types.

        :param other: The model to compare to.
        :type other: Model
        :return: If self comes before other.
        :rtype: bool
        """
        return str(self.t.origin) < str(other.t.origin)

    def is_pmodel(self) -> bool:
        """Check if the model is known to be a PModel, i.e. if all fields have types
        that were known to be PModels.

        Raises an exception if some types are PModels and others not.
        """
        has_ptypes = False
        has_tkrs = False
        for decl in self.decls:
            if decl.t.is_ptype:
                has_ptypes = True
            else:
                has_tkrs = True
        if has_ptypes and has_tkrs:
            raise ValueError("Model decls should be all PTypes or all TKRs")
        return has_ptypes
