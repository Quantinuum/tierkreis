"""Code generated from builtins namespace. Please do not edit."""

from collections.abc import Sequence
from typing import NamedTuple, Union

from tierkreis.controller.data.models import TKR
from tierkreis.controller.data.types import PType


class Headed[T: PType](NamedTuple):
    head: TKR[T]  # fmt: skip
    rest: TKR[list[T]]  # fmt: skip


class Untupled[U: PType, V: PType](NamedTuple):
    a: TKR[U]  # fmt: skip
    b: TKR[V]  # fmt: skip


class Unzipped[U: PType, V: PType](NamedTuple):
    a: TKR[list[U]]  # fmt: skip
    b: TKR[list[V]]  # fmt: skip


class iadd(NamedTuple):
    a: TKR[int]  # fmt: skip
    b: TKR[int]  # fmt: skip

    @staticmethod
    def out() -> type[TKR[int]]:  # fmt: skip
        return TKR[int]  # fmt: skip

    @property
    def namespace(self) -> str:
        return "builtins"


class add(NamedTuple):
    a: TKR[int | float]  # fmt: skip
    b: TKR[int | float]  # fmt: skip

    @staticmethod
    def out() -> type[TKR[int | float]]:  # fmt: skip
        return TKR[Union[int, float]]  # fmt: skip

    @property
    def namespace(self) -> str:
        return "builtins"


class isubtract(NamedTuple):
    a: TKR[int]  # fmt: skip
    b: TKR[int]  # fmt: skip

    @staticmethod
    def out() -> type[TKR[int]]:  # fmt: skip
        return TKR[int]  # fmt: skip

    @property
    def namespace(self) -> str:
        return "builtins"


class subtract(NamedTuple):
    a: TKR[int | float]  # fmt: skip
    b: TKR[int | float]  # fmt: skip

    @staticmethod
    def out() -> type[TKR[int | float]]:  # fmt: skip
        return TKR[Union[int, float]]  # fmt: skip

    @property
    def namespace(self) -> str:
        return "builtins"


class itimes(NamedTuple):
    a: TKR[int]  # fmt: skip
    b: TKR[int]  # fmt: skip

    @staticmethod
    def out() -> type[TKR[int]]:  # fmt: skip
        return TKR[int]  # fmt: skip

    @property
    def namespace(self) -> str:
        return "builtins"


class times(NamedTuple):
    a: TKR[int | float]  # fmt: skip
    b: TKR[int | float]  # fmt: skip

    @staticmethod
    def out() -> type[TKR[int | float]]:  # fmt: skip
        return TKR[Union[int, float]]  # fmt: skip

    @property
    def namespace(self) -> str:
        return "builtins"


class divide(NamedTuple):
    a: TKR[int | float]  # fmt: skip
    b: TKR[int | float]  # fmt: skip

    @staticmethod
    def out() -> type[TKR[float]]:  # fmt: skip
        return TKR[float]  # fmt: skip

    @property
    def namespace(self) -> str:
        return "builtins"


class idivide(NamedTuple):
    a: TKR[int]  # fmt: skip
    b: TKR[int]  # fmt: skip

    @staticmethod
    def out() -> type[TKR[int]]:  # fmt: skip
        return TKR[int]  # fmt: skip

    @property
    def namespace(self) -> str:
        return "builtins"


class igt(NamedTuple):
    a: TKR[int]  # fmt: skip
    b: TKR[int]  # fmt: skip

    @staticmethod
    def out() -> type[TKR[bool]]:  # fmt: skip
        return TKR[bool]  # fmt: skip

    @property
    def namespace(self) -> str:
        return "builtins"


class gt(NamedTuple):
    a: TKR[int | float]  # fmt: skip
    b: TKR[int | float]  # fmt: skip

    @staticmethod
    def out() -> type[TKR[bool]]:  # fmt: skip
        return TKR[bool]  # fmt: skip

    @property
    def namespace(self) -> str:
        return "builtins"


class conjugate(NamedTuple):
    z: TKR[complex]  # fmt: skip

    @staticmethod
    def out() -> type[TKR[complex]]:  # fmt: skip
        return TKR[complex]  # fmt: skip

    @property
    def namespace(self) -> str:
        return "builtins"


class eq(NamedTuple):
    a: TKR[int | float]  # fmt: skip
    b: TKR[int | float]  # fmt: skip

    @staticmethod
    def out() -> type[TKR[bool]]:  # fmt: skip
        return TKR[bool]  # fmt: skip

    @property
    def namespace(self) -> str:
        return "builtins"


class neq(NamedTuple):
    a: TKR[int | float]  # fmt: skip
    b: TKR[int | float]  # fmt: skip

    @staticmethod
    def out() -> type[TKR[bool]]:  # fmt: skip
        return TKR[bool]  # fmt: skip

    @property
    def namespace(self) -> str:
        return "builtins"


class ipow(NamedTuple):
    a: TKR[int]  # fmt: skip
    b: TKR[int]  # fmt: skip

    @staticmethod
    def out() -> type[TKR[int]]:  # fmt: skip
        return TKR[int]  # fmt: skip

    @property
    def namespace(self) -> str:
        return "builtins"


class pow(NamedTuple):
    a: TKR[int | float]  # fmt: skip
    b: TKR[int | float]  # fmt: skip

    @staticmethod
    def out() -> type[TKR[int | float]]:  # fmt: skip
        return TKR[Union[int, float]]  # fmt: skip

    @property
    def namespace(self) -> str:
        return "builtins"


class tkr_abs(NamedTuple):
    a: TKR[int | float]  # fmt: skip

    @staticmethod
    def out() -> type[TKR[int | float]]:  # fmt: skip
        return TKR[Union[int, float]]  # fmt: skip

    @property
    def namespace(self) -> str:
        return "builtins"


class tkr_round(NamedTuple):
    a: TKR[float | int]  # fmt: skip

    @staticmethod
    def out() -> type[TKR[int]]:  # fmt: skip
        return TKR[int]  # fmt: skip

    @property
    def namespace(self) -> str:
        return "builtins"


class neg(NamedTuple):
    a: TKR[bool]  # fmt: skip

    @staticmethod
    def out() -> type[TKR[bool]]:  # fmt: skip
        return TKR[bool]  # fmt: skip

    @property
    def namespace(self) -> str:
        return "builtins"


class trk_and(NamedTuple):
    a: TKR[bool]  # fmt: skip
    b: TKR[bool]  # fmt: skip

    @staticmethod
    def out() -> type[TKR[bool]]:  # fmt: skip
        return TKR[bool]  # fmt: skip

    @property
    def namespace(self) -> str:
        return "builtins"


class trk_or(NamedTuple):
    a: TKR[bool]  # fmt: skip
    b: TKR[bool]  # fmt: skip

    @staticmethod
    def out() -> type[TKR[bool]]:  # fmt: skip
        return TKR[bool]  # fmt: skip

    @property
    def namespace(self) -> str:
        return "builtins"


class tkr_id[T: PType](NamedTuple):
    value: TKR[T]  # fmt: skip

    @staticmethod
    def out() -> type[TKR[T]]:  # fmt: skip
        return TKR[T]  # fmt: skip

    @property
    def namespace(self) -> str:
        return "builtins"


class append[T: PType](NamedTuple):
    v: TKR[list[T]]  # fmt: skip
    a: TKR[T]  # fmt: skip

    @staticmethod
    def out() -> type[TKR[list[T]]]:  # fmt: skip
        return TKR[list[T]]  # fmt: skip

    @property
    def namespace(self) -> str:
        return "builtins"


class head[T: PType](NamedTuple):
    v: TKR[list[T]]  # fmt: skip

    @staticmethod
    def out() -> type[Headed[T]]:  # fmt: skip
        return Headed[T]  # fmt: skip

    @property
    def namespace(self) -> str:
        return "builtins"


class tkr_len[A: PType](NamedTuple):
    v: TKR[list[A]]  # fmt: skip

    @staticmethod
    def out() -> type[TKR[int]]:  # fmt: skip
        return TKR[int]  # fmt: skip

    @property
    def namespace(self) -> str:
        return "builtins"


class str_eq(NamedTuple):
    a: TKR[str]  # fmt: skip
    b: TKR[str]  # fmt: skip

    @staticmethod
    def out() -> type[TKR[bool]]:  # fmt: skip
        return TKR[bool]  # fmt: skip

    @property
    def namespace(self) -> str:
        return "builtins"


class str_neq(NamedTuple):
    a: TKR[str]  # fmt: skip
    b: TKR[str]  # fmt: skip

    @staticmethod
    def out() -> type[TKR[bool]]:  # fmt: skip
        return TKR[bool]  # fmt: skip

    @property
    def namespace(self) -> str:
        return "builtins"


class concat(NamedTuple):
    lhs: TKR[str]  # fmt: skip
    rhs: TKR[str]  # fmt: skip

    @staticmethod
    def out() -> type[TKR[str]]:  # fmt: skip
        return TKR[str]  # fmt: skip

    @property
    def namespace(self) -> str:
        return "builtins"


class tkr_zip[U: PType, V: PType](NamedTuple):
    a: TKR[list[U]]  # fmt: skip
    b: TKR[list[V]]  # fmt: skip

    @staticmethod
    def out() -> type[TKR[list[tuple[U, V]]]]:  # fmt: skip
        return TKR[list[tuple[U, V]]]  # fmt: skip

    @property
    def namespace(self) -> str:
        return "builtins"


class unzip[U: PType, V: PType](NamedTuple):
    value: TKR[list[tuple[U, V]]]  # fmt: skip

    @staticmethod
    def out() -> type[Unzipped[U, V]]:  # fmt: skip
        return Unzipped[U, V]  # fmt: skip

    @property
    def namespace(self) -> str:
        return "builtins"


class tkr_tuple[U: PType, V: PType](NamedTuple):
    a: TKR[U]  # fmt: skip
    b: TKR[V]  # fmt: skip

    @staticmethod
    def out() -> type[TKR[tuple[U, V]]]:  # fmt: skip
        return TKR[tuple[U, V]]  # fmt: skip

    @property
    def namespace(self) -> str:
        return "builtins"


class untuple[U: PType, V: PType](NamedTuple):
    value: TKR[tuple[U, V]]  # fmt: skip

    @staticmethod
    def out() -> type[Untupled[U, V]]:  # fmt: skip
        return Untupled[U, V]  # fmt: skip

    @property
    def namespace(self) -> str:
        return "builtins"


class mean(NamedTuple):
    values: TKR[list[float]]  # fmt: skip

    @staticmethod
    def out() -> type[TKR[float]]:  # fmt: skip
        return TKR[float]  # fmt: skip

    @property
    def namespace(self) -> str:
        return "builtins"


class mod(NamedTuple):
    a: TKR[int]  # fmt: skip
    b: TKR[int]  # fmt: skip

    @staticmethod
    def out() -> type[TKR[int]]:  # fmt: skip
        return TKR[int]  # fmt: skip

    @property
    def namespace(self) -> str:
        return "builtins"


class rand_int(NamedTuple):
    a: TKR[int]  # fmt: skip
    b: TKR[int]  # fmt: skip

    @staticmethod
    def out() -> type[TKR[int]]:  # fmt: skip
        return TKR[int]  # fmt: skip

    @property
    def namespace(self) -> str:
        return "builtins"


class tkr_sleep(NamedTuple):
    delay_seconds: TKR[float]  # fmt: skip

    @staticmethod
    def out() -> type[TKR[bool]]:  # fmt: skip
        return TKR[bool]  # fmt: skip

    @property
    def namespace(self) -> str:
        return "builtins"


class tkr_encode(NamedTuple):
    string: TKR[str]  # fmt: skip

    @staticmethod
    def out() -> type[TKR[bytes]]:  # fmt: skip
        return TKR[bytes]  # fmt: skip

    @property
    def namespace(self) -> str:
        return "builtins"


class tkr_decode(NamedTuple):
    bytes: TKR[bytes]  # fmt: skip

    @staticmethod
    def out() -> type[TKR[str]]:  # fmt: skip
        return TKR[str]  # fmt: skip

    @property
    def namespace(self) -> str:
        return "builtins"


class tkr_all[T: PType](NamedTuple):
    values: TKR[Sequence[T]]  # fmt: skip

    @staticmethod
    def out() -> type[TKR[bool]]:  # fmt: skip
        return TKR[bool]  # fmt: skip

    @property
    def namespace(self) -> str:
        return "builtins"


class tkr_any[T: PType](NamedTuple):
    values: TKR[Sequence[T]]  # fmt: skip

    @staticmethod
    def out() -> type[TKR[bool]]:  # fmt: skip
        return TKR[bool]  # fmt: skip

    @property
    def namespace(self) -> str:
        return "builtins"


class tkr_reversed[T: PType](NamedTuple):
    values: TKR[list[T]]  # fmt: skip

    @staticmethod
    def out() -> type[TKR[list[T]]]:  # fmt: skip
        return TKR[list[T]]  # fmt: skip

    @property
    def namespace(self) -> str:
        return "builtins"


class tkr_extend[T: PType](NamedTuple):
    first: TKR[list[T]]  # fmt: skip
    second: TKR[list[T]]  # fmt: skip

    @staticmethod
    def out() -> type[TKR[list[T]]]:  # fmt: skip
        return TKR[list[T]]  # fmt: skip

    @property
    def namespace(self) -> str:
        return "builtins"


class concat_lists[U: PType, V: PType](NamedTuple):
    first: TKR[list[U]]  # fmt: skip
    second: TKR[list[V]]  # fmt: skip

    @staticmethod
    def out() -> type[TKR[list[U | V]]]:  # fmt: skip
        return TKR[list[U | V]]  # fmt: skip

    @property
    def namespace(self) -> str:
        return "builtins"


class tkr_str(NamedTuple):
    value: TKR[int | float | bool]  # fmt: skip

    @staticmethod
    def out() -> type[TKR[str]]:  # fmt: skip
        return TKR[str]  # fmt: skip

    @property
    def namespace(self) -> str:
        return "builtins"


class tkr_int(NamedTuple):
    value: TKR[int | float | bool | str]  # fmt: skip

    @staticmethod
    def out() -> type[TKR[int]]:  # fmt: skip
        return TKR[int]  # fmt: skip

    @property
    def namespace(self) -> str:
        return "builtins"


class sum_list(NamedTuple):
    values: TKR[list[int | float]]  # fmt: skip

    @staticmethod
    def out() -> type[TKR[int | float]]:  # fmt: skip
        return TKR[Union[int, float]]  # fmt: skip

    @property
    def namespace(self) -> str:
        return "builtins"


class prod_list(NamedTuple):
    values: TKR[list[int | float]]  # fmt: skip

    @staticmethod
    def out() -> type[TKR[int | float]]:  # fmt: skip
        return TKR[Union[int, float]]  # fmt: skip

    @property
    def namespace(self) -> str:
        return "builtins"


class max_item(NamedTuple):
    values: TKR[list[int | float]]  # fmt: skip

    @staticmethod
    def out() -> type[TKR[int | float]]:  # fmt: skip
        return TKR[Union[int, float]]  # fmt: skip

    @property
    def namespace(self) -> str:
        return "builtins"


class min_item(NamedTuple):
    values: TKR[list[int | float]]  # fmt: skip

    @staticmethod
    def out() -> type[TKR[int | float]]:  # fmt: skip
        return TKR[Union[int, float]]  # fmt: skip

    @property
    def namespace(self) -> str:
        return "builtins"


class sort_number_list(NamedTuple):
    values: TKR[list[int | float]]  # fmt: skip

    @staticmethod
    def out() -> type[TKR[list[int | float]]]:  # fmt: skip
        return TKR[list[int | float]]  # fmt: skip

    @property
    def namespace(self) -> str:
        return "builtins"


class sort_string_list(NamedTuple):
    values: TKR[list[str]]  # fmt: skip

    @staticmethod
    def out() -> type[TKR[list[str]]]:  # fmt: skip
        return TKR[list[str]]  # fmt: skip

    @property
    def namespace(self) -> str:
        return "builtins"


class flatten[T: PType](NamedTuple):
    values: TKR[list[list[T]]]  # fmt: skip

    @staticmethod
    def out() -> type[TKR[list[T]]]:  # fmt: skip
        return TKR[list[T]]  # fmt: skip

    @property
    def namespace(self) -> str:
        return "builtins"


class take[T: PType](NamedTuple):
    values: TKR[list[T]]  # fmt: skip
    n: TKR[int]  # fmt: skip

    @staticmethod
    def out() -> type[TKR[list[T]]]:  # fmt: skip
        return TKR[list[T]]  # fmt: skip

    @property
    def namespace(self) -> str:
        return "builtins"


class drop[T: PType](NamedTuple):
    values: TKR[list[T]]  # fmt: skip
    n: TKR[int]  # fmt: skip

    @staticmethod
    def out() -> type[TKR[list[T]]]:  # fmt: skip
        return TKR[list[T]]  # fmt: skip

    @property
    def namespace(self) -> str:
        return "builtins"
