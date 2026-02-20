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
    include_bound: bool,
    is_tkr: bool,
) -> str:
    bound_str = ": PType" if include_bound else ""
    if isinstance(generictype, str):
        out = generictype + bound_str
        return f"TKR[{out}]" if is_tkr else out

    origin_str = format_ptype(generictype.origin)

    generics = [format_generic_type(x, include_bound, False) for x in generictype.args]
    generics_str = f"[{', '.join(generics)}]" if generictype.args else ""

    out = f"{origin_str}{generics_str}"
    return f"TKR[{out}]" if is_tkr else out


def format_typed_arg(typed_arg: TypedArg, is_portmaping: bool) -> str:
    type_str = format_generic_type(typed_arg.t, False, not is_portmaping)
    should_quote = typed_arg.t.included_structs() and is_portmaping
    type_str = f'"{type_str}"' if should_quote else type_str
    default_str = " | None = None " if typed_arg.has_default else ""
    return f"{typed_arg.name}: {type_str}{default_str} {NO_QA_STR}"


def format_model(model: Model) -> str:
    is_portmapping = model.is_portmapping
    outs = [format_typed_arg(x, not is_portmapping) for x in model.decls]
    outs.sort()
    outs_str = "\n    ".join(outs)

    bases = ["NamedTuple"] if is_portmapping else ["Struct", "Protocol"]

    return f"""
class {format_generic_type(model.t, True, False)}({", ".join(bases)}):
    {outs_str}
"""


def format_method(namespace_name: str, fn: Method) -> str:
    ins = [format_typed_arg(x, False) for x in fn.args]
    ins_str = "\n    ".join(ins)
    class_name = format_generic_type(
        fn.return_type,
        False,
        not fn.return_type_is_portmapping,
    )

    bases = ["NamedTuple"]

    return f"""class {format_generic_type(fn.name, True, False)}({", ".join(bases)}):
    {ins_str}

    @staticmethod
    def out() -> type[{class_name}]:{NO_QA_STR}
        return {class_name}{NO_QA_STR}

    @property
    def namespace(self) -> str:
        return "{namespace_name}" """
