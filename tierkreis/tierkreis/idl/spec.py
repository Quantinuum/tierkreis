"""Combinators for the Tierkreis worker IDL.

We use
https://typespec.io/docs/language-basics/models/
https://typespec.io/docs/language-basics/interfaces/
as well as the extra decorators
- @portmapping on models with multiple outputs,
- @file("name") on `bytes` ports staged as files for shell workers,
- @args on `Array<string>` inputs passed as a bash array to shell workers.
"""

from tierkreis.exceptions import TierkreisError
from tierkreis.idl.models import GenericType, Interface, Method, Model, TypedArg
from tierkreis.idl.parser import lit, reg, seq
from tierkreis.idl.type_symbols import generic_t, ident, type_symbol

string_literal = reg(r'"[^"\\]*"').map(lambda x: x[1:-1])


def _file_option(name: str) -> tuple[str, str | None]:
    return ("file", name)


file_decorator = (lit("@file(") >> string_literal << lit(")")).map(_file_option)
args_option: tuple[str, str | None] = ("args", None)
args_decorator = lit("@args").coerce(args_option)
decorator = file_decorator | args_decorator


def _typed_arg(
    x: tuple[list[tuple[str, str | None]], str, GenericType],
) -> TypedArg:
    decorators, name, t = x
    options: dict[str, str | None] = {}
    for key, value in decorators:
        if key in options:
            msg = f"Duplicate @{key} on {name}"
            raise TierkreisError(msg)
        options[key] = value
    return TypedArg(name, t, file=options.get("file"), args="args" in options)


type_decl = seq(decorator.rep(), ident << lit(":"), type_symbol).map(_typed_arg)
model = seq(
    lit("@portmapping").opt().map(lambda x: x is not None) << lit("model"),
    generic_t,
    lit("{") >> type_decl.rep(lit(";")) << lit("}"),
).map(lambda x: Model(*x))
method = seq(
    generic_t,
    lit("(") >> type_decl.rep(lit(",")) << lit(")") << lit(":"),
    type_symbol,
).map(lambda x: Method(*x))
interface = (
    (lit("interface") >> ident << lit("{")) & method.rep(lit(";")) << lit("}")
).map(lambda x: Interface(*x))
spec = model.rep() & interface
