"""Typed graph-building stubs for the Rust builtin worker."""

from collections.abc import Sequence
from typing import NamedTuple

from tierkreis.controller.data.models import TKR
from tierkreis.controller.data.types import PType


class Headed[T: PType](NamedTuple):
    head: TKR[T]
    rest: TKR[list[T]]


class Untupled[U: PType, V: PType](NamedTuple):
    a: TKR[U]
    b: TKR[V]


class Unzipped[U: PType, V: PType](NamedTuple):
    a: TKR[list[U]]
    b: TKR[list[V]]


class iadd(NamedTuple):
    a: TKR[int]
    b: TKR[int]

    @staticmethod
    def out() -> type[TKR[int]]:
        return TKR[int]

    @property
    def namespace(self) -> str:
        return "builtins"


class add(NamedTuple):
    a: TKR[float]
    b: TKR[float]

    @staticmethod
    def out() -> type[TKR[int | float]]:
        return TKR[int | float]

    @property
    def namespace(self) -> str:
        return "builtins"


class isubtract(NamedTuple):
    a: TKR[int]
    b: TKR[int]

    @staticmethod
    def out() -> type[TKR[int]]:
        return TKR[int]

    @property
    def namespace(self) -> str:
        return "builtins"


class subtract(NamedTuple):
    a: TKR[float]
    b: TKR[float]

    @staticmethod
    def out() -> type[TKR[int | float]]:
        return TKR[int | float]

    @property
    def namespace(self) -> str:
        return "builtins"


class itimes(NamedTuple):
    a: TKR[int]
    b: TKR[int]

    @staticmethod
    def out() -> type[TKR[int]]:
        return TKR[int]

    @property
    def namespace(self) -> str:
        return "builtins"


class times(NamedTuple):
    a: TKR[float]
    b: TKR[float]

    @staticmethod
    def out() -> type[TKR[int | float]]:
        return TKR[int | float]

    @property
    def namespace(self) -> str:
        return "builtins"


class divide(NamedTuple):
    a: TKR[float]
    b: TKR[float]

    @staticmethod
    def out() -> type[TKR[float]]:
        return TKR[float]

    @property
    def namespace(self) -> str:
        return "builtins"


class idivide(NamedTuple):
    a: TKR[int]
    b: TKR[int]

    @staticmethod
    def out() -> type[TKR[int]]:
        return TKR[int]

    @property
    def namespace(self) -> str:
        return "builtins"


class igt(NamedTuple):
    a: TKR[int]
    b: TKR[int]

    @staticmethod
    def out() -> type[TKR[bool]]:
        return TKR[bool]

    @property
    def namespace(self) -> str:
        return "builtins"


class gt(NamedTuple):
    a: TKR[float]
    b: TKR[float]

    @staticmethod
    def out() -> type[TKR[bool]]:
        return TKR[bool]

    @property
    def namespace(self) -> str:
        return "builtins"


class lt(NamedTuple):
    a: TKR[float]
    b: TKR[float]

    @staticmethod
    def out() -> type[TKR[bool]]:
        return TKR[bool]

    @property
    def namespace(self) -> str:
        return "builtins"


class conjugate(NamedTuple):
    z: TKR[complex]

    @staticmethod
    def out() -> type[TKR[complex]]:
        return TKR[complex]

    @property
    def namespace(self) -> str:
        return "builtins"


class eq(NamedTuple):
    a: TKR[float]
    b: TKR[float]

    @staticmethod
    def out() -> type[TKR[bool]]:
        return TKR[bool]

    @property
    def namespace(self) -> str:
        return "builtins"


class neq(NamedTuple):
    a: TKR[float]
    b: TKR[float]

    @staticmethod
    def out() -> type[TKR[bool]]:
        return TKR[bool]

    @property
    def namespace(self) -> str:
        return "builtins"


class ipow(NamedTuple):
    a: TKR[int]
    b: TKR[int]

    @staticmethod
    def out() -> type[TKR[int]]:
        return TKR[int]

    @property
    def namespace(self) -> str:
        return "builtins"


class tkr_pow(NamedTuple):
    a: TKR[float]
    b: TKR[float]

    @staticmethod
    def out() -> type[TKR[int | float]]:
        return TKR[int | float]

    @property
    def namespace(self) -> str:
        return "builtins"


class tkr_abs(NamedTuple):
    a: TKR[float]

    @staticmethod
    def out() -> type[TKR[int | float]]:
        return TKR[int | float]

    @property
    def namespace(self) -> str:
        return "builtins"


class tkr_round(NamedTuple):
    a: TKR[float]

    @staticmethod
    def out() -> type[TKR[int]]:
        return TKR[int]

    @property
    def namespace(self) -> str:
        return "builtins"


class neg(NamedTuple):
    a: TKR[bool]

    @staticmethod
    def out() -> type[TKR[bool]]:
        return TKR[bool]

    @property
    def namespace(self) -> str:
        return "builtins"


class tkr_and(NamedTuple):
    a: TKR[bool]
    b: TKR[bool]

    @staticmethod
    def out() -> type[TKR[bool]]:
        return TKR[bool]

    @property
    def namespace(self) -> str:
        return "builtins"


class tkr_or(NamedTuple):
    a: TKR[bool]
    b: TKR[bool]

    @staticmethod
    def out() -> type[TKR[bool]]:
        return TKR[bool]

    @property
    def namespace(self) -> str:
        return "builtins"


class tkr_id[T: PType](NamedTuple):
    value: TKR[T]

    @staticmethod
    def out() -> type[TKR[T]]:
        return TKR[T]

    @property
    def namespace(self) -> str:
        return "builtins"


class append[T: PType](NamedTuple):
    v: TKR[list[T]]
    a: TKR[T]

    @staticmethod
    def out() -> type[TKR[list[T]]]:
        return TKR[list[T]]

    @property
    def namespace(self) -> str:
        return "builtins"


class head[T: PType](NamedTuple):
    v: TKR[list[T]]

    @staticmethod
    def out() -> type[Headed[T]]:
        return Headed[T]

    @property
    def namespace(self) -> str:
        return "builtins"


class tkr_len[A: PType](NamedTuple):
    v: TKR[list[A]]

    @staticmethod
    def out() -> type[TKR[int]]:
        return TKR[int]

    @property
    def namespace(self) -> str:
        return "builtins"


class str_eq(NamedTuple):
    a: TKR[str]
    b: TKR[str]

    @staticmethod
    def out() -> type[TKR[bool]]:
        return TKR[bool]

    @property
    def namespace(self) -> str:
        return "builtins"


class str_neq(NamedTuple):
    a: TKR[str]
    b: TKR[str]

    @staticmethod
    def out() -> type[TKR[bool]]:
        return TKR[bool]

    @property
    def namespace(self) -> str:
        return "builtins"


class concat(NamedTuple):
    lhs: TKR[str]
    rhs: TKR[str]

    @staticmethod
    def out() -> type[TKR[str]]:
        return TKR[str]

    @property
    def namespace(self) -> str:
        return "builtins"


class tkr_zip[U: PType, V: PType](NamedTuple):
    a: TKR[list[U]]
    b: TKR[list[V]]

    @staticmethod
    def out() -> type[TKR[list[tuple[U, V]]]]:
        return TKR[list[tuple[U, V]]]

    @property
    def namespace(self) -> str:
        return "builtins"


class unzip[U: PType, V: PType](NamedTuple):
    value: TKR[list[tuple[U, V]]]

    @staticmethod
    def out() -> type[Unzipped[U, V]]:
        return Unzipped[U, V]

    @property
    def namespace(self) -> str:
        return "builtins"


class tkr_tuple[U: PType, V: PType](NamedTuple):
    a: TKR[U]
    b: TKR[V]

    @staticmethod
    def out() -> type[TKR[tuple[U, V]]]:
        return TKR[tuple[U, V]]

    @property
    def namespace(self) -> str:
        return "builtins"


class untuple[U: PType, V: PType](NamedTuple):
    value: TKR[tuple[U, V]]

    @staticmethod
    def out() -> type[Untupled[U, V]]:
        return Untupled[U, V]

    @property
    def namespace(self) -> str:
        return "builtins"


class mean(NamedTuple):
    values: TKR[list[float]]

    @staticmethod
    def out() -> type[TKR[float]]:
        return TKR[float]

    @property
    def namespace(self) -> str:
        return "builtins"


class mod(NamedTuple):
    a: TKR[int]
    b: TKR[int]

    @staticmethod
    def out() -> type[TKR[int]]:
        return TKR[int]

    @property
    def namespace(self) -> str:
        return "builtins"


class tkr_range(NamedTuple):
    start: TKR[int]
    stop: TKR[int]
    step: TKR[int] | None = None

    @staticmethod
    def out() -> type[TKR[list[int]]]:
        return TKR[list[int]]

    @property
    def namespace(self) -> str:
        return "builtins"


class rand_int(NamedTuple):
    a: TKR[int]
    b: TKR[int]

    @staticmethod
    def out() -> type[TKR[int]]:
        return TKR[int]

    @property
    def namespace(self) -> str:
        return "builtins"


class tkr_sleep(NamedTuple):
    delay_seconds: TKR[float]

    @staticmethod
    def out() -> type[TKR[bool]]:
        return TKR[bool]

    @property
    def namespace(self) -> str:
        return "builtins"


class tkr_encode(NamedTuple):
    string: TKR[str]

    @staticmethod
    def out() -> type[TKR[bytes]]:
        return TKR[bytes]

    @property
    def namespace(self) -> str:
        return "builtins"


class tkr_decode(NamedTuple):
    value_bytes: TKR[bytes]

    @staticmethod
    def out() -> type[TKR[str]]:
        return TKR[str]

    @property
    def namespace(self) -> str:
        return "builtins"


class tkr_all[T: PType](NamedTuple):
    values: TKR[Sequence[T]]

    @staticmethod
    def out() -> type[TKR[bool]]:
        return TKR[bool]

    @property
    def namespace(self) -> str:
        return "builtins"


class tkr_any[T: PType](NamedTuple):
    values: TKR[Sequence[T]]

    @staticmethod
    def out() -> type[TKR[bool]]:
        return TKR[bool]

    @property
    def namespace(self) -> str:
        return "builtins"


class tkr_reversed[T: PType](NamedTuple):
    values: TKR[list[T]]

    @staticmethod
    def out() -> type[TKR[list[T]]]:
        return TKR[list[T]]

    @property
    def namespace(self) -> str:
        return "builtins"


class tkr_extend[T: PType](NamedTuple):
    first: TKR[list[T]]
    second: TKR[list[T]]

    @staticmethod
    def out() -> type[TKR[list[T]]]:
        return TKR[list[T]]

    @property
    def namespace(self) -> str:
        return "builtins"


class concat_lists[U: PType, V: PType](NamedTuple):
    first: TKR[list[U]]
    second: TKR[list[V]]

    @staticmethod
    def out() -> type[TKR[list[U | V]]]:
        return TKR[list[U | V]]

    @property
    def namespace(self) -> str:
        return "builtins"


class tkr_str(NamedTuple):
    value: TKR[float | bool]

    @staticmethod
    def out() -> type[TKR[str]]:
        return TKR[str]

    @property
    def namespace(self) -> str:
        return "builtins"


class tkr_int(NamedTuple):
    value: TKR[float | bool | str]

    @staticmethod
    def out() -> type[TKR[int]]:
        return TKR[int]

    @property
    def namespace(self) -> str:
        return "builtins"


class sum_list(NamedTuple):
    values: TKR[list[int | float]]

    @staticmethod
    def out() -> type[TKR[int | float]]:
        return TKR[int | float]

    @property
    def namespace(self) -> str:
        return "builtins"


class prod_list(NamedTuple):
    values: TKR[list[int | float]]

    @staticmethod
    def out() -> type[TKR[int | float]]:
        return TKR[int | float]

    @property
    def namespace(self) -> str:
        return "builtins"


class max_item(NamedTuple):
    values: TKR[list[int | float]]

    @staticmethod
    def out() -> type[TKR[int | float]]:
        return TKR[int | float]

    @property
    def namespace(self) -> str:
        return "builtins"


class min_item(NamedTuple):
    values: TKR[list[int | float]]

    @staticmethod
    def out() -> type[TKR[int | float]]:
        return TKR[int | float]

    @property
    def namespace(self) -> str:
        return "builtins"


class sort_number_list(NamedTuple):
    values: TKR[list[int | float]]

    @staticmethod
    def out() -> type[TKR[list[int | float]]]:
        return TKR[list[int | float]]

    @property
    def namespace(self) -> str:
        return "builtins"


class sort_string_list(NamedTuple):
    values: TKR[list[str]]

    @staticmethod
    def out() -> type[TKR[list[str]]]:
        return TKR[list[str]]

    @property
    def namespace(self) -> str:
        return "builtins"


class flatten[T: PType](NamedTuple):
    values: TKR[list[list[T]]]

    @staticmethod
    def out() -> type[TKR[list[T]]]:
        return TKR[list[T]]

    @property
    def namespace(self) -> str:
        return "builtins"


class take[T: PType](NamedTuple):
    values: TKR[list[T]]
    n: TKR[int]

    @staticmethod
    def out() -> type[TKR[list[T]]]:
        return TKR[list[T]]

    @property
    def namespace(self) -> str:
        return "builtins"


class drop[T: PType](NamedTuple):
    values: TKR[list[T]]
    n: TKR[int]

    @staticmethod
    def out() -> type[TKR[list[T]]]:
        return TKR[list[T]]

    @property
    def namespace(self) -> str:
        return "builtins"


class at[T: PType](NamedTuple):
    values: TKR[list[T]]
    n: TKR[int]

    @staticmethod
    def out() -> type[TKR[T]]:
        return TKR[T]

    @property
    def namespace(self) -> str:
        return "builtins"


# Rust-native builtin names and legacy aliases. Subclassing the generated
# signatures preserves their static constructor and output types while giving
# each task the class name used to construct its worker call.
class ieq(igt):
    pass


class ine(igt):
    pass


class ilt_s(igt):
    pass


class igt_s(igt):
    pass


class ilte_s(igt):
    pass


class igte_s(igt):
    pass


class imax_s(iadd):
    pass


class imin_s(iadd):
    pass


class isub(iadd):
    pass


class ineg(NamedTuple):
    a: TKR[int]

    @staticmethod
    def out() -> type[TKR[int]]:
        return TKR[int]

    @property
    def namespace(self) -> str:
        return "builtins"


class imul(iadd):
    pass


class idiv_s(iadd):
    pass


class imod_s(iadd):
    pass


class iabs(ineg):
    pass


class iand(iadd):
    pass


class ior(iadd):
    pass


class ixor(iadd):
    pass


class itostring_s(NamedTuple):
    a: TKR[int]

    @staticmethod
    def out() -> type[TKR[str]]:
        return TKR[str]

    @property
    def namespace(self) -> str:
        return "builtins"


class fadd(NamedTuple):
    a: TKR[float]
    b: TKR[float]

    @staticmethod
    def out() -> type[TKR[float]]:
        return TKR[float]

    @property
    def namespace(self) -> str:
        return "builtins"


class fsub(fadd):
    pass


class fmul(fadd):
    pass


class fmax(fadd):
    pass


class fmin(fadd):
    pass


class idiv(fadd):
    pass


class feq(gt):
    pass


class fne(gt):
    pass


class flt(gt):
    pass


class fgt(gt):
    pass


class flte(gt):
    pass


class fgte(gt):
    pass


class fneg(NamedTuple):
    a: TKR[float]

    @staticmethod
    def out() -> type[TKR[float]]:
        return TKR[float]

    @property
    def namespace(self) -> str:
        return "builtins"


class fabs(fneg):
    pass


class ffloor(fneg):
    pass


class fceil(fneg):
    pass


class ftostring(NamedTuple):
    a: TKR[float]

    @staticmethod
    def out() -> type[TKR[str]]:
        return TKR[str]

    @property
    def namespace(self) -> str:
        return "builtins"


class ne(neq):
    pass


class _str(tkr_str):
    pass


_str.__name__ = "str"


class tuple[U: PType, V: PType](tkr_tuple[U, V]):
    pass


class range(tkr_range):
    pass


class sleep(tkr_sleep):
    pass
