"""Code generated from builtins namespace. Please do not edit."""

from typing import NamedTuple, Sequence, Union
from tierkreis.controller.data.models import TKR
from tierkreis.controller.data.types import PType


class Headed[T: PType](NamedTuple):
    head: TKR[T]  # noqa: F821 # fmt: skip
    rest: TKR[list[T]]  # noqa: F821 # fmt: skip


class Untupled[U: PType, V: PType](NamedTuple):
    a: TKR[U]  # noqa: F821 # fmt: skip
    b: TKR[V]  # noqa: F821 # fmt: skip


class Unzipped[U: PType, V: PType](NamedTuple):
    a: TKR[list[U]]  # noqa: F821 # fmt: skip
    b: TKR[list[V]]  # noqa: F821 # fmt: skip


class iadd(NamedTuple):
    a: TKR[int]  # noqa: F821 # fmt: skip
    b: TKR[int]  # noqa: F821 # fmt: skip

    @staticmethod
    def out() -> type[TKR[int]]:  # noqa: F821 # fmt: skip
        return TKR[int]  # noqa: F821 # fmt: skip

    @property
    def namespace(self) -> str:
        return "builtins"


class add(NamedTuple):
    a: TKR[float]  # fmt: skip
    b: TKR[float]  # fmt: skip

    @staticmethod
    def out() -> type[TKR[Union[int, float]]]:  # fmt: skip
        return TKR[Union[int, float]]  # fmt: skip

    @property
    def namespace(self) -> str:
        return "builtins"


class isubtract(NamedTuple):
    a: TKR[int]  # noqa: F821 # fmt: skip
    b: TKR[int]  # noqa: F821 # fmt: skip

    @staticmethod
    def out() -> type[TKR[int]]:  # noqa: F821 # fmt: skip
        return TKR[int]  # noqa: F821 # fmt: skip

    @property
    def namespace(self) -> str:
        return "builtins"


class subtract(NamedTuple):
    a: TKR[float]  # fmt: skip
    b: TKR[float]  # fmt: skip

    @staticmethod
    def out() -> type[TKR[Union[int, float]]]:  # fmt: skip
        return TKR[Union[int, float]]  # fmt: skip

    @property
    def namespace(self) -> str:
        return "builtins"


class itimes(NamedTuple):
    a: TKR[int]  # noqa: F821 # fmt: skip
    b: TKR[int]  # noqa: F821 # fmt: skip

    @staticmethod
    def out() -> type[TKR[int]]:  # noqa: F821 # fmt: skip
        return TKR[int]  # noqa: F821 # fmt: skip

    @property
    def namespace(self) -> str:
        return "builtins"


class times(NamedTuple):
    a: TKR[float]  # fmt: skip
    b: TKR[float]  # fmt: skip

    @staticmethod
    def out() -> type[TKR[Union[int, float]]]:  # fmt: skip
        return TKR[Union[int, float]]  # fmt: skip

    @property
    def namespace(self) -> str:
        return "builtins"


class divide(NamedTuple):
    a: TKR[float]  # fmt: skip
    b: TKR[float]  # fmt: skip

    @staticmethod
    def out() -> type[TKR[float]]:  # noqa: F821 # fmt: skip
        return TKR[float]  # noqa: F821 # fmt: skip

    @property
    def namespace(self) -> str:
        return "builtins"


class idivide(NamedTuple):
    a: TKR[int]  # noqa: F821 # fmt: skip
    b: TKR[int]  # noqa: F821 # fmt: skip

    @staticmethod
    def out() -> type[TKR[int]]:  # noqa: F821 # fmt: skip
        return TKR[int]  # noqa: F821 # fmt: skip

    @property
    def namespace(self) -> str:
        return "builtins"


class igt(NamedTuple):
    a: TKR[int]  # noqa: F821 # fmt: skip
    b: TKR[int]  # noqa: F821 # fmt: skip

    @staticmethod
    def out() -> type[TKR[bool]]:  # noqa: F821 # fmt: skip
        return TKR[bool]  # noqa: F821 # fmt: skip

    @property
    def namespace(self) -> str:
        return "builtins"


class gt(NamedTuple):
    a: TKR[float]  # fmt: skip
    b: TKR[float]  # fmt: skip

    @staticmethod
    def out() -> type[TKR[bool]]:  # fmt: skip
        return TKR[bool]  # fmt: skip

    @property
    def namespace(self) -> str:
        return "builtins"


class lt(NamedTuple):
    a: TKR[float]  # fmt: skip
    b: TKR[float]  # fmt: skip

    @staticmethod
    def out() -> type[TKR[bool]]:  # noqa: F821 # fmt: skip
        return TKR[bool]  # noqa: F821 # fmt: skip

    @property
    def namespace(self) -> str:
        return "builtins"


class conjugate(NamedTuple):
    z: TKR[complex]  # noqa: F821 # fmt: skip

    @staticmethod
    def out() -> type[TKR[complex]]:  # noqa: F821 # fmt: skip
        return TKR[complex]  # noqa: F821 # fmt: skip

    @property
    def namespace(self) -> str:
        return "builtins"


class eq(NamedTuple):
    a: TKR[float]  # fmt: skip
    b: TKR[float]  # fmt: skip

    @staticmethod
    def out() -> type[TKR[bool]]:  # noqa: F821 # fmt: skip
        return TKR[bool]  # noqa: F821 # fmt: skip

    @property
    def namespace(self) -> str:
        return "builtins"


class neq(NamedTuple):
    a: TKR[float]  # fmt: skip
    b: TKR[float]  # fmt: skip

    @staticmethod
    def out() -> type[TKR[bool]]:  # noqa: F821 # fmt: skip
        return TKR[bool]  # noqa: F821 # fmt: skip

    @property
    def namespace(self) -> str:
        return "builtins"


class ipow(NamedTuple):
    a: TKR[int]  # noqa: F821 # fmt: skip
    b: TKR[int]  # noqa: F821 # fmt: skip

    @staticmethod
    def out() -> type[TKR[int]]:  # noqa: F821 # fmt: skip
        return TKR[int]  # noqa: F821 # fmt: skip

    @property
    def namespace(self) -> str:
        return "builtins"


class tkr_pow(NamedTuple):
    a: TKR[float]  # fmt: skip
    b: TKR[float]  # fmt: skip

    @staticmethod
    def out() -> type[TKR[Union[int, float]]]:  # fmt: skip
        return TKR[Union[int, float]]  # fmt: skip

    @property
    def namespace(self) -> str:
        return "builtins"


class tkr_abs(NamedTuple):
    a: TKR[float]  # fmt: skip

    @staticmethod
    def out() -> type[TKR[Union[int, float]]]:  # fmt: skip
        return TKR[Union[int, float]]  # fmt: skip

    @property
    def namespace(self) -> str:
        return "builtins"


class tkr_round(NamedTuple):
    a: TKR[float]  # fmt: skip

    @staticmethod
    def out() -> type[TKR[int]]:  # noqa: F821 # fmt: skip
        return TKR[int]  # noqa: F821 # fmt: skip

    @property
    def namespace(self) -> str:
        return "builtins"


class neg(NamedTuple):
    a: TKR[bool]  # noqa: F821 # fmt: skip

    @staticmethod
    def out() -> type[TKR[bool]]:  # noqa: F821 # fmt: skip
        return TKR[bool]  # noqa: F821 # fmt: skip

    @property
    def namespace(self) -> str:
        return "builtins"


class tkr_and(NamedTuple):
    a: TKR[bool]  # fmt: skip
    b: TKR[bool]  # fmt: skip

    @staticmethod
    def out() -> type[TKR[bool]]:  # noqa: F821 # fmt: skip
        return TKR[bool]  # noqa: F821 # fmt: skip

    @property
    def namespace(self) -> str:
        return "builtins"


class tkr_or(NamedTuple):
    a: TKR[bool]  # fmt: skip
    b: TKR[bool]  # fmt: skip

    @staticmethod
    def out() -> type[TKR[bool]]:  # noqa: F821 # fmt: skip
        return TKR[bool]  # noqa: F821 # fmt: skip

    @property
    def namespace(self) -> str:
        return "builtins"


class tkr_id[T: PType](NamedTuple):
    value: TKR[T]  # noqa: F821 # fmt: skip

    @staticmethod
    def out() -> type[TKR[T]]:  # noqa: F821 # fmt: skip
        return TKR[T]  # noqa: F821 # fmt: skip

    @property
    def namespace(self) -> str:
        return "builtins"


class append[T: PType](NamedTuple):
    v: TKR[list[T]]  # noqa: F821 # fmt: skip
    a: TKR[T]  # noqa: F821 # fmt: skip

    @staticmethod
    def out() -> type[TKR[list[T]]]:  # noqa: F821 # fmt: skip
        return TKR[list[T]]  # noqa: F821 # fmt: skip

    @property
    def namespace(self) -> str:
        return "builtins"


class head[T: PType](NamedTuple):
    v: TKR[list[T]]  # noqa: F821 # fmt: skip

    @staticmethod
    def out() -> type[Headed[T]]:  # noqa: F821 # fmt: skip
        return Headed[T]  # noqa: F821 # fmt: skip

    @property
    def namespace(self) -> str:
        return "builtins"


class tkr_len[A: PType](NamedTuple):
    v: TKR[list[A]]  # noqa: F821 # fmt: skip

    @staticmethod
    def out() -> type[TKR[int]]:  # noqa: F821 # fmt: skip
        return TKR[int]  # noqa: F821 # fmt: skip

    @property
    def namespace(self) -> str:
        return "builtins"


class str_eq(NamedTuple):
    a: TKR[str]  # noqa: F821 # fmt: skip
    b: TKR[str]  # noqa: F821 # fmt: skip

    @staticmethod
    def out() -> type[TKR[bool]]:  # noqa: F821 # fmt: skip
        return TKR[bool]  # noqa: F821 # fmt: skip

    @property
    def namespace(self) -> str:
        return "builtins"


class str_neq(NamedTuple):
    a: TKR[str]  # noqa: F821 # fmt: skip
    b: TKR[str]  # noqa: F821 # fmt: skip

    @staticmethod
    def out() -> type[TKR[bool]]:  # noqa: F821 # fmt: skip
        return TKR[bool]  # noqa: F821 # fmt: skip

    @property
    def namespace(self) -> str:
        return "builtins"


class concat(NamedTuple):
    lhs: TKR[str]  # noqa: F821 # fmt: skip
    rhs: TKR[str]  # noqa: F821 # fmt: skip

    @staticmethod
    def out() -> type[TKR[str]]:  # noqa: F821 # fmt: skip
        return TKR[str]  # noqa: F821 # fmt: skip

    @property
    def namespace(self) -> str:
        return "builtins"


class tkr_zip[U: PType, V: PType](NamedTuple):
    a: TKR[list[U]]  # noqa: F821 # fmt: skip
    b: TKR[list[V]]  # noqa: F821 # fmt: skip

    @staticmethod
    def out() -> type[TKR[list[tuple[U, V]]]]:  # noqa: F821 # fmt: skip
        return TKR[list[tuple[U, V]]]  # noqa: F821 # fmt: skip

    @property
    def namespace(self) -> str:
        return "builtins"


class unzip[U: PType, V: PType](NamedTuple):
    value: TKR[list[tuple[U, V]]]  # noqa: F821 # fmt: skip

    @staticmethod
    def out() -> type[Unzipped[U, V]]:  # noqa: F821 # fmt: skip
        return Unzipped[U, V]  # noqa: F821 # fmt: skip

    @property
    def namespace(self) -> str:
        return "builtins"


class tkr_tuple[U: PType, V: PType](NamedTuple):
    a: TKR[U]  # noqa: F821 # fmt: skip
    b: TKR[V]  # noqa: F821 # fmt: skip

    @staticmethod
    def out() -> type[TKR[tuple[U, V]]]:  # noqa: F821 # fmt: skip
        return TKR[tuple[U, V]]  # noqa: F821 # fmt: skip

    @property
    def namespace(self) -> str:
        return "builtins"


class untuple[U: PType, V: PType](NamedTuple):
    value: TKR[tuple[U, V]]  # noqa: F821 # fmt: skip

    @staticmethod
    def out() -> type[Untupled[U, V]]:  # noqa: F821 # fmt: skip
        return Untupled[U, V]  # noqa: F821 # fmt: skip

    @property
    def namespace(self) -> str:
        return "builtins"


class mean(NamedTuple):
    values: TKR[list[float]]  # noqa: F821 # fmt: skip

    @staticmethod
    def out() -> type[TKR[float]]:  # noqa: F821 # fmt: skip
        return TKR[float]  # noqa: F821 # fmt: skip

    @property
    def namespace(self) -> str:
        return "builtins"


class mod(NamedTuple):
    a: TKR[int]  # noqa: F821 # fmt: skip
    b: TKR[int]  # noqa: F821 # fmt: skip

    @staticmethod
    def out() -> type[TKR[int]]:  # noqa: F821 # fmt: skip
        return TKR[int]  # noqa: F821 # fmt: skip

    @property
    def namespace(self) -> str:
        return "builtins"


class tkr_range(NamedTuple):
    start: TKR[int]  # noqa: F821 # fmt: skip
    stop: TKR[int]  # noqa: F821 # fmt: skip
    step: TKR[int] | None = None  # noqa: F821 # fmt: skip

    @staticmethod
    def out() -> type[TKR[list[int]]]:  # noqa: F821 # fmt: skip
        return TKR[list[int]]  # noqa: F821 # fmt: skip

    @property
    def namespace(self) -> str:
        return "builtins"


class rand_int(NamedTuple):
    a: TKR[int]  # noqa: F821 # fmt: skip
    b: TKR[int]  # noqa: F821 # fmt: skip

    @staticmethod
    def out() -> type[TKR[int]]:  # noqa: F821 # fmt: skip
        return TKR[int]  # noqa: F821 # fmt: skip

    @property
    def namespace(self) -> str:
        return "builtins"


class tkr_sleep(NamedTuple):
    delay_seconds: TKR[float]  # noqa: F821 # fmt: skip

    @staticmethod
    def out() -> type[TKR[bool]]:  # noqa: F821 # fmt: skip
        return TKR[bool]  # noqa: F821 # fmt: skip

    @property
    def namespace(self) -> str:
        return "builtins"


class tkr_encode(NamedTuple):
    string: TKR[str]  # noqa: F821 # fmt: skip

    @staticmethod
    def out() -> type[TKR[bytes]]:  # noqa: F821 # fmt: skip
        return TKR[bytes]  # noqa: F821 # fmt: skip

    @property
    def namespace(self) -> str:
        return "builtins"


class tkr_decode(NamedTuple):
    value_bytes: TKR[bytes]  # fmt: skip

    @staticmethod
    def out() -> type[TKR[str]]:  # noqa: F821 # fmt: skip
        return TKR[str]  # noqa: F821 # fmt: skip

    @property
    def namespace(self) -> str:
        return "builtins"


class tkr_all[T: PType](NamedTuple):
    values: TKR[Sequence[T]]  # noqa: F821 # fmt: skip

    @staticmethod
    def out() -> type[TKR[bool]]:  # noqa: F821 # fmt: skip
        return TKR[bool]  # noqa: F821 # fmt: skip

    @property
    def namespace(self) -> str:
        return "builtins"


class tkr_any[T: PType](NamedTuple):
    values: TKR[Sequence[T]]  # noqa: F821 # fmt: skip

    @staticmethod
    def out() -> type[TKR[bool]]:  # noqa: F821 # fmt: skip
        return TKR[bool]  # noqa: F821 # fmt: skip

    @property
    def namespace(self) -> str:
        return "builtins"


class tkr_reversed[T: PType](NamedTuple):
    values: TKR[list[T]]  # noqa: F821 # fmt: skip

    @staticmethod
    def out() -> type[TKR[list[T]]]:  # noqa: F821 # fmt: skip
        return TKR[list[T]]  # noqa: F821 # fmt: skip

    @property
    def namespace(self) -> str:
        return "builtins"


class tkr_extend[T: PType](NamedTuple):
    first: TKR[list[T]]  # noqa: F821 # fmt: skip
    second: TKR[list[T]]  # noqa: F821 # fmt: skip

    @staticmethod
    def out() -> type[TKR[list[T]]]:  # noqa: F821 # fmt: skip
        return TKR[list[T]]  # noqa: F821 # fmt: skip

    @property
    def namespace(self) -> str:
        return "builtins"


class concat_lists[U: PType, V: PType](NamedTuple):
    first: TKR[list[U]]  # noqa: F821 # fmt: skip
    second: TKR[list[V]]  # noqa: F821 # fmt: skip

    @staticmethod
    def out() -> type[TKR[list[Union[U, V]]]]:  # fmt: skip
        return TKR[list[Union[U, V]]]  # fmt: skip

    @property
    def namespace(self) -> str:
        return "builtins"


class tkr_str(NamedTuple):
    value: TKR[Union[float, bool]]  # fmt: skip

    @staticmethod
    def out() -> type[TKR[str]]:  # noqa: F821 # fmt: skip
        return TKR[str]  # noqa: F821 # fmt: skip

    @property
    def namespace(self) -> str:
        return "builtins"


class tkr_int(NamedTuple):
    value: TKR[Union[float, bool, str]]  # fmt: skip

    @staticmethod
    def out() -> type[TKR[int]]:  # noqa: F821 # fmt: skip
        return TKR[int]  # noqa: F821 # fmt: skip

    @property
    def namespace(self) -> str:
        return "builtins"


class sum_list(NamedTuple):
    values: TKR[list[Union[int, float]]]  # fmt: skip

    @staticmethod
    def out() -> type[TKR[Union[int, float]]]:  # fmt: skip
        return TKR[Union[int, float]]  # fmt: skip

    @property
    def namespace(self) -> str:
        return "builtins"


class prod_list(NamedTuple):
    values: TKR[list[Union[int, float]]]  # fmt: skip

    @staticmethod
    def out() -> type[TKR[Union[int, float]]]:  # fmt: skip
        return TKR[Union[int, float]]  # fmt: skip

    @property
    def namespace(self) -> str:
        return "builtins"


class max_item(NamedTuple):
    values: TKR[list[Union[int, float]]]  # fmt: skip

    @staticmethod
    def out() -> type[TKR[Union[int, float]]]:  # fmt: skip
        return TKR[Union[int, float]]  # fmt: skip

    @property
    def namespace(self) -> str:
        return "builtins"


class min_item(NamedTuple):
    values: TKR[list[Union[int, float]]]  # fmt: skip

    @staticmethod
    def out() -> type[TKR[Union[int, float]]]:  # fmt: skip
        return TKR[Union[int, float]]  # fmt: skip

    @property
    def namespace(self) -> str:
        return "builtins"


class sort_number_list(NamedTuple):
    values: TKR[list[Union[int, float]]]  # fmt: skip

    @staticmethod
    def out() -> type[TKR[list[Union[int, float]]]]:  # fmt: skip
        return TKR[list[Union[int, float]]]  # fmt: skip

    @property
    def namespace(self) -> str:
        return "builtins"


class sort_string_list(NamedTuple):
    values: TKR[list[str]]  # noqa: F821 # fmt: skip

    @staticmethod
    def out() -> type[TKR[list[str]]]:  # noqa: F821 # fmt: skip
        return TKR[list[str]]  # noqa: F821 # fmt: skip

    @property
    def namespace(self) -> str:
        return "builtins"


class flatten[T: PType](NamedTuple):
    values: TKR[list[list[T]]]  # noqa: F821 # fmt: skip

    @staticmethod
    def out() -> type[TKR[list[T]]]:  # noqa: F821 # fmt: skip
        return TKR[list[T]]  # noqa: F821 # fmt: skip

    @property
    def namespace(self) -> str:
        return "builtins"


class take[T: PType](NamedTuple):
    values: TKR[list[T]]  # noqa: F821 # fmt: skip
    n: TKR[int]  # noqa: F821 # fmt: skip

    @staticmethod
    def out() -> type[TKR[list[T]]]:  # noqa: F821 # fmt: skip
        return TKR[list[T]]  # noqa: F821 # fmt: skip

    @property
    def namespace(self) -> str:
        return "builtins"


class drop[T: PType](NamedTuple):
    values: TKR[list[T]]  # noqa: F821 # fmt: skip
    n: TKR[int]  # noqa: F821 # fmt: skip

    @staticmethod
    def out() -> type[TKR[list[T]]]:  # noqa: F821 # fmt: skip
        return TKR[list[T]]  # noqa: F821 # fmt: skip

    @property
    def namespace(self) -> str:
        return "builtins"
