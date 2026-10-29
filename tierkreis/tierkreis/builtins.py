"""Typed graph-building stubs for the builtin worker."""

import builtins as py_builtins
from typing import NamedTuple

from tierkreis.controller.data.models import TKR
from tierkreis.controller.data.types import PType


class Untupled[U: PType, V: PType](NamedTuple):
    a: TKR[U]
    b: TKR[V]


# i64 operations
class ieq(NamedTuple):
    a: TKR[int]
    b: TKR[int]

    @staticmethod
    def out() -> type[TKR[bool]]:
        return TKR[bool]

    @property
    def namespace(self) -> str:
        return "builtins"


class ine(ieq):
    pass


class ilt_s(ieq):
    pass


class igt_s(ieq):
    pass


class igt(ieq):
    pass


class ilte_s(ieq):
    pass


class igte_s(ieq):
    pass


class imax_s(NamedTuple):
    a: TKR[int]
    b: TKR[int]

    @staticmethod
    def out() -> type[TKR[int]]:
        return TKR[int]

    @property
    def namespace(self) -> str:
        return "builtins"


class imin_s(imax_s):
    pass


class iadd(imax_s):
    pass


class isub(imax_s):
    pass


class ineg(NamedTuple):
    a: TKR[int]

    @staticmethod
    def out() -> type[TKR[int]]:
        return TKR[int]

    @property
    def namespace(self) -> str:
        return "builtins"


class imul(imax_s):
    pass


class itimes(imax_s):
    pass


class idiv_s(imax_s):
    pass


class idivide(imax_s):
    pass


class imod_s(imax_s):
    pass


class mod(imax_s):
    pass


class iabs(ineg):
    pass


class iand(imax_s):
    pass


class ior(imax_s):
    pass


class ixor(imax_s):
    pass


class itostring_s(NamedTuple):
    a: TKR[int]

    @staticmethod
    def out() -> type[TKR[str]]:
        return TKR[str]

    @property
    def namespace(self) -> str:
        return "builtins"


# f64 operations
class feq(NamedTuple):
    a: TKR[float]
    b: TKR[float]

    @staticmethod
    def out() -> type[TKR[bool]]:
        return TKR[bool]

    @property
    def namespace(self) -> str:
        return "builtins"


class fne(feq):
    pass


class flt(feq):
    pass


class fgt(feq):
    pass


class flte(feq):
    pass


class fgte(feq):
    pass


class fmax(NamedTuple):
    a: TKR[float]
    b: TKR[float]

    @staticmethod
    def out() -> type[TKR[float]]:
        return TKR[float]

    @property
    def namespace(self) -> str:
        return "builtins"


class fmin(fmax):
    pass


class fadd(fmax):
    pass


class fsub(fmax):
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


class fmul(fmax):
    pass


class idiv(fmax):
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


# Value operations
class eq(NamedTuple):
    a: TKR[float]
    b: TKR[float]

    @staticmethod
    def out() -> type[TKR[bool]]:
        return TKR[bool]

    @property
    def namespace(self) -> str:
        return "builtins"


class ne(eq):
    pass


class neq(eq):
    pass


class _str(NamedTuple):
    value: TKR[float | bool]

    @staticmethod
    def out() -> type[TKR[str]]:
        return TKR[str]

    @property
    def namespace(self) -> str:
        return "builtins"


_str.__name__ = "str"


class tkr_str(_str):
    pass


class str_eq(NamedTuple):
    a: TKR[py_builtins.str]
    b: TKR[py_builtins.str]

    @staticmethod
    def out() -> type[TKR[bool]]:
        return TKR[bool]

    @property
    def namespace(self) -> py_builtins.str:
        return "builtins"


class tuple[U: PType, V: PType](NamedTuple):
    a: TKR[U]
    b: TKR[V]

    @staticmethod
    def out() -> type[TKR[py_builtins.tuple[U, V]]]:
        return TKR[py_builtins.tuple[U, V]]

    @property
    def namespace(self) -> py_builtins.str:
        return "builtins"


class tkr_tuple[U: PType, V: PType](tuple[U, V]):
    pass


class untuple[U: PType, V: PType](NamedTuple):
    value: TKR[py_builtins.tuple[U, V]]

    @staticmethod
    def out() -> type[Untupled[U, V]]:
        return Untupled[U, V]

    @property
    def namespace(self) -> py_builtins.str:
        return "builtins"


class mean(NamedTuple):
    values: TKR[list[float]]

    @staticmethod
    def out() -> type[TKR[float]]:
        return TKR[float]

    @property
    def namespace(self) -> py_builtins.str:
        return "builtins"


class range(NamedTuple):
    start: TKR[int]
    stop: TKR[int]
    step: TKR[int] | None = None

    @staticmethod
    def out() -> type[TKR[list[int]]]:
        return TKR[list[int]]

    @property
    def namespace(self) -> py_builtins.str:
        return "builtins"


class tkr_range(range):
    pass


class conjugate(NamedTuple):
    z: TKR[complex]

    @staticmethod
    def out() -> type[TKR[complex]]:
        return TKR[complex]

    @property
    def namespace(self) -> py_builtins.str:
        return "builtins"


class sleep(NamedTuple):
    delay_seconds: TKR[float]

    @staticmethod
    def out() -> type[TKR[bool]]:
        return TKR[bool]

    @property
    def namespace(self) -> py_builtins.str:
        return "builtins"


class rand_int(NamedTuple):
    a: TKR[int]
    b: TKR[int]

    @staticmethod
    def out() -> type[TKR[int]]:
        return TKR[int]

    @property
    def namespace(self) -> py_builtins.str:
        return "builtins"


class sum_list(NamedTuple):
    values: TKR[list[int | float]]

    @staticmethod
    def out() -> type[TKR[int | float]]:
        return TKR[int | float]

    @property
    def namespace(self) -> py_builtins.str:
        return "builtins"


class prod_list(sum_list):
    pass
