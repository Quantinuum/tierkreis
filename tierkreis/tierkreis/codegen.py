"""Code generation utilities for Tierkreis stubs."""

from inspect import isclass

from pydantic import BaseModel

from tierkreis.controller.data.types import (
    DictConvertible,
    ListConvertible,
    NdarraySurrogate,
    _is_union,
)
from tierkreis.idl.models import GenericType, Method, Model, TypedArg

NO_QA_STR = " # noqa: F821 # fmt: skip"


def format_ptype(ptype: type | str) -> str:
    """Format a ptype to a string.

    :param ptype: The type to format.
    :type ptype: type | str
    :return: The formatted string representation of the type.
    :rtype: str
    """
    if isinstance(ptype, str):
        return ptype

    if isclass(ptype) and issubclass(
        ptype,
        (DictConvertible, ListConvertible, NdarraySurrogate, BaseModel),
    ):
        return f'OpaqueType["{ptype.__module__}.{ptype.__qualname__}"]'

    if _is_union(ptype):
        return "Union"

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

    origin_str = format_ptype(generictype.origin)

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
    should_quote = typed_arg.t.included_structs() and is_portmapping
    type_str = f'"{type_str}"' if should_quote else type_str
    default_str = " | None = None " if typed_arg.has_default else ""
    return f"{typed_arg.name}: {type_str}{default_str} {NO_QA_STR}"


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

    def is_tmodel() -> bool:
        has_ptypes = False
        has_tkrs = False
        for decl in model.decls:
            if decl.t.is_ptype:
                has_ptypes = True
            else:
                has_tkrs = True
        assert not (has_ptypes and has_tkrs), (
            "Model decls should be all PTypes or all TKRs"
        )
        return has_tkrs

    bases = ["NamedTuple"] if is_portmapping or is_tmodel() else ["Struct", "Protocol"]
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
    def out() -> type[{class_name}]:{NO_QA_STR}
        return {class_name}{NO_QA_STR}

    @property
    def namespace(self) -> str:
        return "{namespace_name}" """
