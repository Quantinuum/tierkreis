"""Types that are allowed in the Tierkreis worker specification IDL.

We use https://typespec.io/docs/language-basics/built-in-types/ as a guide.
"""

from types import NoneType
from typing import ForwardRef

from tierkreis.idl.models import GenericType
from tierkreis.idl.parser import Parser, lit, reg, seq

type _TypeT = type | ForwardRef


signed_int = lit("integer", "int64", "int32", "int16", "int8", "safeint")
unsigned_int = lit("uint64", "uint32", "uint16", "uint8")
integer_t = (signed_int | unsigned_int).coerce(GenericType(int, []))
float_t = lit("float", "float32", "float64", "numeric").coerce(GenericType(float, []))
bytes_t = lit("bytes").coerce(GenericType(bytes, []))
string_t = lit("string", "url").coerce(GenericType(str, []))
bool_t = lit("bool").coerce(GenericType(bool, []))
none_t = lit("null").coerce(GenericType(NoneType, []))
decimal_t = lit("decimal", "decimal128").fail("Decimal")
plain_datetime = lit("plainDate", "plainTime")
other_date = lit("utcDateTime", "offsetDateTime", "duration")
date_t = (plain_datetime | other_date).fail("Date")
unknown_t = lit("unknown", "void", "never").fail("Unknown")
ident = reg(r"[a-zA-Z0-9_]+")
forward_ref = ident.map(ForwardRef)
generics = (lit("<") >> ident.rep(lit(",")) << lit(">")).opt().map(lambda x: x or [])


def array_t(ins: str) -> tuple[GenericType, str]:
    return (lit("Array<") >> type_symbol << lit(">")).map(
        lambda x: GenericType(list, [x]),
    )(ins)


def record_t(ins: str) -> tuple[GenericType, str]:
    return (lit("Record<") >> type_symbol << lit(">")).map(
        lambda x: GenericType(dict, [GenericType(str, []), x]),
    )(ins)


@Parser
def generic_t(ins: str) -> tuple[GenericType, str]:
    return seq(
        ident,
        (lit("<") >> ident.rep(lit(",")) << lit(">")).opt().map(lambda x: x or []),
    ).map(lambda x: GenericType(x[0], x[1]))(ins)


@Parser
def type_symbol(ins: str) -> tuple[GenericType, str]:
    return (
        integer_t
        | float_t
        | bytes_t
        | bool_t
        | none_t
        | string_t
        | decimal_t
        | unknown_t
        | date_t
        | array_t
        | record_t
        | generic_t
    )(ins)
