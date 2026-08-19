"""Code generation utilities for Tierkreis stubs."""

from inspect import isclass
from types import NoneType
from typing import ForwardRef

from hugr.package import Package
from pydantic import BaseModel

from tierkreis.controller.data.types import (
    DictConvertible,
    ListConvertible,
    NdarraySurrogate,
    _is_union,
)
from tierkreis.idl.models import GenericType, Method, Model, TypedArg


def format_ptype(
    ptype: type | str | ForwardRef, *, has_serialization: bool = False
) -> str:
    """Format a ptype to a string.

    :param ptype: The type to format.
    :type ptype: type | str | ForwardRef
    :param has_serialization: If it was a custom serialized type.
    :type has_serialization: bool, defaults to False.
    :return: The formatted string representation of the type.
    :rtype: str
    """
    if isinstance(ptype, str):
        return ptype

    if isclass(ptype) and issubclass(
        ptype,
        (DictConvertible, ListConvertible, NdarraySurrogate, BaseModel, Package),
    ):
        return f'OpaqueType["{ptype.__module__}.{ptype.__qualname__}"]'
    if ptype is NoneType or ptype is None:
        return "NoneType"
    if has_serialization:
        return f'OpaqueType["{ptype.__module__}.{ptype.__qualname__}"]'
    if isinstance(ptype, ForwardRef):
        return ptype.__forward_arg__
    return ptype.__qualname__


def format_generic_type(
    generictype: GenericType | str,
    *,
    include_bound: bool,
    is_tkr: bool,
) -> str:
    """Format a generic type to a string.

    :param generictype: The generic type to format.
    :type generictype: GenericType | str
    :param include_bound: Whether to include the bound.
    :type include_bound: bool
    :param is_tkr: Whether the type is a TKR type.
    :type is_tkr: bool
    :return: The formatted string representation of the generic type.
    :rtype: str
    """
    bound_str = ": PType" if include_bound else ""
    if isinstance(generictype, str):
        out = generictype + bound_str
        return f"TKR[{out}]" if is_tkr else out

    if _is_union(generictype.origin):
        variants = [
            format_generic_type(x, include_bound=include_bound, is_tkr=False)
            for x in generictype.args
        ]
        out = " | ".join(variants)
        return f"TKR[{out}]" if is_tkr else out

    origin_str = format_ptype(
        generictype.origin, has_serialization=generictype.has_serialization
    )

    generics = [
        format_generic_type(x, include_bound=include_bound, is_tkr=False)
        for x in generictype.args
    ]
    generics_str = f"[{', '.join(generics)}]" if generictype.args else ""

    out = f"{origin_str}{generics_str}"
    return f"TKR[{out}]" if is_tkr else out


def format_typed_arg(typed_arg: TypedArg, *, is_portmapping: bool) -> str:
    """Format a typed argument to a string.

    :param typed_arg: The typed argument.
    :type typed_arg: TypedArg
    :param is_portmapping: Whether the argument is a portmapping.
    :type is_portmapping: bool
    :return: The formatted string representation of the typed argument.
    :rtype: str
    """
    type_str = format_generic_type(
        typed_arg.t,
        include_bound=False,
        is_tkr=not is_portmapping,
    )

    default_str = " | None = None " if typed_arg.has_default else ""
    return f"{typed_arg.name}: {type_str}{default_str}"


def format_model(model: Model) -> str:
    """Format a model to a string.

    :param model: The model to format.
    :type model: Model
    :return: The formatted string representation of the model.
    :rtype: str
    """
    is_portmapping = model.is_portmapping
    outs = [format_typed_arg(x, is_portmapping=not is_portmapping) for x in model.decls]
    outs.sort()
    outs_str = "\n    ".join(outs)

    bases = (
        ["NamedTuple"]
        if is_portmapping or not model.is_pmodel()
        else ["Struct", "Protocol"]
    )
    bases_str = ", ".join(bases)
    generic_type_str = format_generic_type(model.t, include_bound=True, is_tkr=False)

    return f"""
class {generic_type_str}({bases_str}):
    {outs_str}
"""


def format_method(namespace_name: str, fn: Method) -> str:
    """Format a method to a string.

    :param namespace_name: The function namespace.
    :type namespace_name: str
    :param fn: The method to format.
    :type fn: Method
    :return: The formatted string representation of the method.
    :rtype: str
    """
    ins = [format_typed_arg(x, is_portmapping=False) for x in fn.args]
    ins_str = "\n    ".join(ins)
    class_name = format_generic_type(
        fn.return_type,
        include_bound=False,
        is_tkr=not fn.return_type_is_portmapping,
    )

    bases = ["NamedTuple"]

    class_def = format_generic_type(fn.name, include_bound=True, is_tkr=False)
    bases_str = ", ".join(bases)

    return f"""class {class_def}({bases_str}):
    {ins_str}

    @staticmethod
    def out() -> type[{class_name}]:
        return {class_name}

    @property
    def namespace(self) -> str:
        return "{namespace_name}" """
