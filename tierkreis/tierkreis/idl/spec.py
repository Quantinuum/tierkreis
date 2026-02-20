"""Combinators for the Tierkreis worker IDL.

We use
https://typespec.io/docs/language-basics/models/
https://typespec.io/docs/language-basics/interfaces/
as well as an extra decorator @portmapping.
"""

from tierkreis.idl.models import Interface, Method, Model, TypedArg
from tierkreis.idl.parser import lit, seq
from tierkreis.idl.type_symbols import generic_t, ident, type_symbol

type_decl = ((ident << lit(":")) & type_symbol).map(lambda x: TypedArg(*x))
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
