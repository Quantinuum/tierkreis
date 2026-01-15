"""Built-in Tierkreis tasks for basic operations."""

import statistics
from collections.abc import Sequence
from logging import getLogger
from pathlib import Path
from random import randint
from sys import argv
from time import sleep
from typing import NamedTuple

from tierkreis import Worker
from tierkreis.controller.data.location import WorkerCallArgs
from tierkreis.controller.data.models import portmapping
from tierkreis.controller.data.types import (
    PType,
    bytes_from_ptype,
    ptype_from_bytes,
)
from tierkreis.worker.storage.protocol import WorkerStorage
from tierkreis.worker.worker import TierkreisWorkerError

logger = getLogger(__name__)

worker = Worker("builtins")


@worker.task()
def iadd(a: int, b: int) -> int:
    """Add two integers a+b.

    :param a: The first integer.
    :type a: int
    :param b: The second integer.
    :type b: int
    :return: The sum of the two integers.
    :rtype: int
    """
    logger.debug("iadd %s %s", a, b)
    return a + b


@worker.task()
def add(a: float, b: float) -> int | float:
    """Add two float like values a+b.

    Returns an int if both inputs are integers, otherwise a float.

    :param a: The first value.
    :type a: float
    :param b: The second value.
    :type b: float
    :return: The sum of the two values.
    :rtype: int | float
    """
    return a + b


@worker.task()
def isubtract(a: int, b: int) -> int:
    """Subtract two integers a-b.

    :param a: The first integer.
    :type a: int
    :param b: The second integer.
    :type b: int
    :return: The difference of the two integers.
    :rtype: int
    """
    return a - b


@worker.task()
def subtract(a: float, b: float) -> int | float:
    """Subtract two float like values a-b.

    Returns an int if both inputs are integers, otherwise a float.

    :param a: The first value.
    :type a: float
    :param b: The second value.
    :type b: float
    :return: The difference of the two values.
    :rtype: int | float
    """
    return a - b


@worker.task()
def itimes(a: int, b: int) -> int:
    """Multiply two integers a*b.

    :param a: The first integer.
    :type a: int
    :param b: The second integer.
    :type b: int
    :return: The product of the two integers.
    :rtype: int
    """
    logger.debug("itimes %s %s", a, b)
    return a * b


@worker.task()
def times(a: float, b: float) -> int | float:
    """Multiply two float like values a*b.

    Returns an int if both inputs are integers, otherwise a float.

    :param a: The first value.
    :type a: float
    :param b: The second value.
    :type b: float
    :return: The product of the two values.
    :rtype: int | float
    """
    return a * b


@worker.task()
def divide(a: float, b: float) -> float:
    """Divide two float like values a/b.

    :param a: The dividend.
    :type a: float
    :param b: The divisor.
    :type b: float
    :return: The quotient of the two values.
    :rtype: float
    """
    return a / b


@worker.task()
def idivide(a: int, b: int) -> int:
    """Integer division of two integers a//b.

    :param a: The dividend.
    :type a: int
    :param b: The divisor.
    :type b: int
    :return: The integer quotient of the two integers.
    :rtype: int
    """
    return a // b


@worker.task()
def igt(a: int, b: int) -> bool:
    """Check if integer a is greater than integer b.

    :param a: The first integer.
    :type a: int
    :param b: The second integer.
    :type b: int
    :return: True if a > b, False otherwise.
    :rtype: bool
    """
    logger.debug("igt %s %s", a, b)
    return a > b


@worker.task()
def gt(a: float, b: float) -> bool:
    """Check if value a is greater than value b.

    :param a: The first value.
    :type a: float
    :param b: The second value.
    :type b: float
    :return: True if a > b, False otherwise.
    :rtype: bool
    """
    return a > b


@worker.task()
def lt(a: float, b: float) -> bool:
    """Check if value a is less than value b.

    :param a: The first value.
    :type a: float
    :param b: The second value.
    :type b: float
    :return: True if a < b, False otherwise.
    :rtype: bool
    """
    return a < b


@worker.task()
def conjugate(z: complex) -> complex:
    """Return the complex conjugate of z.

    :param z: The complex number.
    :type z: complex
    :return: The complex conjugate of z.
    :rtype: complex
    """
    return z.conjugate()


@worker.task()
def eq(a: float, b: float) -> bool:
    """Check if two float like values are equal.

    :param a: The first value.
    :type a: float
    :param b: The second value.
    :type b: float
    :return: True if a == b, False otherwise.
    :rtype: bool
    """
    return a == b


@worker.task()
def neq(a: float, b: float) -> bool:
    """Check if two float like values are not equal.

    :param a: The first value.
    :type a: float
    :param b: The second value.
    :type b: float
    :return: True if a != b, False otherwise.
    :rtype: bool
    """
    return a != b


@worker.task()
def ipow(a: int, b: int) -> int:
    """Raise integer a to the power of integer b.

    :param a: The base integer.
    :type a: int
    :param b: The exponent integer.
    :type b: int
    :return: The result of a**b.
    :rtype: int
    """
    return a**b


@worker.task()
def tkr_pow(a: float, b: float) -> int | float:
    """Raise value a to the power of value b.

    Returns an int if both inputs are integers, otherwise a float.

    :param a: The base value.
    :type a: float
    :param b: The exponent value.
    :type b: float
    :return: The result of a**b.
    :rtype: int | float
    """
    return a**b


@worker.task()
def tkr_abs(a: float) -> int | float:
    """Return the absolute value of a float like value.

    :param a: The value.
    :type a: float
    :return: The absolute value of a.
    :rtype: int | float
    """
    return abs(a)


@worker.task()
def tkr_round(a: float) -> int:
    """Round a float to the nearest integer.

    :param a: The float value to round.
    :type a: float
    :return: The rounded integer.
    :rtype: int
    """
    return round(a)


@worker.task()
def neg(*, a: bool) -> bool:
    """Negate a boolean value.

    :param a: The boolean value.
    :type a: bool
    :return: The negated boolean value.
    :rtype: bool
    """
    return not a


@worker.task()
def trk_and(*, a: bool, b: bool) -> bool:
    """Return the logical AND of two boolean values.

    :param a: The first boolean value.
    :type a: bool
    :param b: The second boolean value.
    :type b: bool
    :return: The logical AND of a and b.
    :rtype: bool
    """
    logger.debug("and %s %s", a, b)
    return a and b


@worker.task()
def trk_or(*, a: bool, b: bool) -> bool:
    """Return the logical OR of two boolean values.

    :param a: The first boolean value.
    :type a: bool
    :param b: The second boolean value.
    :type b: bool
    :return: The logical OR of a and b.
    :rtype: bool
    """
    logger.debug("and %s %s", a, b)
    return a or b


@worker.task()
def tkr_id[T: PType](value: T) -> T:
    """Return the input value unchanged (identity function).

    :param value: The value to return.
    :type value: T
    :return: The same value.
    :rtype: T
    """
    logger.debug("id %s", value)
    return value


@worker.task()
def append[T](v: list[T], a: T) -> list[T]:
    """Append an element to a list and return the modified list.

    :param v: The list to append to.
    :type v: list[T]
    :param a: The element to append.
    :type a: T
    :return: The list with the element appended.
    :rtype: list[T]
    """
    v.append(a)
    return v


@portmapping
class Headed[T: PType](NamedTuple):
    """A tuple containing a head element and the rest of the list."""

    head: T
    rest: list[T]


@worker.task()
def head[T: PType](v: list[T]) -> Headed[T]:
    """Return the first element and remaining elements of a list.

    :param v: The list.
    :type v: list[T]
    :return: A Headed tuple containing the first element and the rest of the list.
    :rtype: Headed[T]
    """
    head, rest = v[0], v[1:]
    return Headed(head=head, rest=rest)


@worker.task()
def tkr_len[A](v: list[A]) -> int:
    """Return the length of a list.

    :param v: The list.
    :type v: list[A]
    :return: The number of elements in the list.
    :rtype: int
    """
    logger.info("len: %s", v)
    return len(v)


@worker.task()
def str_eq(a: str, b: str) -> bool:
    """Check if two strings are equal.

    :param a: The first string.
    :type a: str
    :param b: The second string.
    :type b: str
    :return: True if the strings are equal, False otherwise.
    :rtype: bool
    """
    return a == b


@worker.task()
def str_neq(a: str, b: str) -> bool:
    """Check if two strings are not equal.

    :param a: The first string.
    :type a: str
    :param b: The second string.
    :type b: str
    :return: True if the strings are not equal, False otherwise.
    :rtype: bool
    """
    return a != b


@worker.primitive_task()
def fold_values(args: WorkerCallArgs, storage: WorkerStorage) -> None:
    """Fold multiple values from storage into a single list.

    Reads values from storage matching a glob pattern (values_glob)
    and combines them into a single list output at the specified output path.

    :param args: The worker call arguments containing the glob pattern and output path.
    :type args: WorkerCallArgs
    :param storage: The worker storage for reading and writing values.
    :type storage: WorkerStorage
    """
    values_glob = storage.glob(str(args.inputs["values_glob"]))
    values_glob.sort(key=lambda x: int(Path(x).name.split("-")[-1]))
    bs = [storage.read_input(Path(value)) for value in values_glob]
    values = [ptype_from_bytes(b) for b in bs]
    storage.write_output(Path(args.outputs["value"]), bytes_from_ptype(values))


@worker.primitive_task()
def unfold_values(args: WorkerCallArgs, storage: WorkerStorage) -> None:
    """Unfold a single list value into multiple individual values in storage.

    Reads a list from storage and writes each element to a separate storage location.

    :param args: The worker call arguments containing input value and output directory.
    :type args: WorkerCallArgs
    :param storage: The worker storage for reading and writing values.
    :type storage: WorkerStorage
    :raises TierkreisWorkerError: If the input is not a list or sequence.
    """
    value_list = ptype_from_bytes(storage.read_input(args.inputs["value"]))
    match value_list:
        case list() | Sequence():
            for i, v in enumerate(value_list):
                storage.write_output(args.output_dir / str(i), bytes_from_ptype(v))
        case _:
            msg = f"Expected list found {value_list}"
            raise TierkreisWorkerError(msg)


@worker.task()
def concat(lhs: str, rhs: str) -> str:
    """Concatenate two strings lhs+rhs.

    :param lhs: The first string.
    :type lhs: str
    :param rhs: The second string.
    :type rhs: str
    :return: The concatenated string.
    :rtype: str
    """
    return lhs + rhs


@worker.task()
def tkr_zip[U, V](a: list[U], b: list[V]) -> list[tuple[U, V]]:
    """Zip two lists together into a list of tuples.

    :param a: The first list.
    :type a: list[U]
    :param b: The second list.
    :type b: list[V]
    :return: A list of tuples pairing elements from both lists.
    :rtype: list[tuple[U, V]]
    """
    return list(zip(a, b, strict=False))


@portmapping
class Unzipped[U: PType, V: PType](NamedTuple):
    """A tuple containing two lists resulting from unzipping."""

    a: list[U]
    b: list[V]


@worker.task()
def unzip[U: PType, V: PType](value: list[tuple[U, V]]) -> Unzipped[U, V]:
    """Unzip a list of tuples into two separate lists.

    :param value: The list of tuples to unzip.
    :type value: list[tuple[U, V]]
    :return: An Unzipped tuple containing two lists.
    :rtype: Unzipped[U, V]
    """
    value_a, value_b = map(list, zip(*value, strict=False))
    return Unzipped(a=value_a, b=value_b)


@worker.task()
def tkr_tuple[U, V](a: U, b: V) -> tuple[U, V]:
    """Create a tuple from two values.

    :param a: The first value.
    :type a: U
    :param b: The second value.
    :type b: V
    :return: A tuple containing both values.
    :rtype: tuple[U, V]
    """
    return (a, b)


@portmapping
class Untupled[U: PType, V: PType](NamedTuple):
    """A tuple containing two unpacked values."""

    a: U
    b: V


@worker.task()
def untuple[U: PType, V: PType](value: tuple[U, V]) -> Untupled[U, V]:
    """Unpack a tuple of two elements into separate values.

    :param value: The tuple to unpack.
    :type value: tuple[U, V]
    :return: An Untupled tuple containing the two unpacked values.
    :rtype: Untupled[U, V]
    """
    logger.info("untuple: %s", value)
    value_a, value_b = value
    return Untupled(a=value_a, b=value_b)


@worker.task()
def mean(values: list[float]) -> float:
    """Calculate the arithmetic mean of a list of floats.

    :param values: The list of float values.
    :type values: list[float]
    :return: The mean of the values.
    :rtype: float
    """
    return statistics.mean(values)


@worker.task()
def mod(a: int, b: int) -> int:
    """Return the modulo of two integers a % b.

    :param a: The dividend.
    :type a: int
    :param b: The divisor.
    :type b: int
    :return: The remainder of a divided by b.
    :rtype: int
    """
    return a % b


@worker.task()
def tkr_range(start: int, stop: int, step: int = 1) -> list[int]:
    return list(range(start, stop, step))


@worker.task()
def rand_int(a: int, b: int) -> int:
    """Return a random integer between a and b (inclusive).

    :param a: The lower bound (inclusive).
    :type a: int
    :param b: The upper bound (inclusive).
    :type b: int
    :return: A random integer between a and b.
    :rtype: int
    """
    return randint(a, b)  # noqa: S311


@worker.task()
def tkr_sleep(delay_seconds: float) -> bool:
    """Sleep for a specified number of seconds.

    :param delay_seconds: The number of seconds to sleep.
    :type delay_seconds: float
    :return: True after the sleep completes.
    :rtype: bool
    """
    sleep(delay_seconds)
    return True


@worker.task()
def tkr_encode(string: str) -> bytes:
    """Encode a string to bytes using UTF-8 encoding.

    :param string: The string to encode.
    :type string: str
    :return: The UTF-8 encoded bytes.
    :rtype: bytes
    """
    return string.encode()


@worker.task()
def tkr_decode(value_bytes: bytes) -> str:
    """Decode bytes to a string using UTF-8 decoding.

    :param value_bytes: The bytes to decode.
    :type value_bytes: bytes
    :return: The decoded string.
    :rtype: str
    """
    return value_bytes.decode()


@worker.task()
def tkr_all[T: PType](values: Sequence[T]) -> bool:
    """Check if all elements in a sequence are truthy.

    :param values: The sequence of values.
    :type values: Sequence[T]
    :return: True if all elements are truthy, False otherwise.
    :rtype: bool
    """
    return all(values)


@worker.task()
def tkr_any[T: PType](values: Sequence[T]) -> bool:
    """Check if any element in a sequence is truthy.

    :param values: The sequence of values.
    :type values: Sequence[T]
    :return: True if any element is truthy, False otherwise.
    :rtype: bool
    """
    return any(values)


@worker.task()
def tkr_reversed[T: PType](values: list[T]) -> list[T]:
    """Return a reversed copy of a list.

    :param values: The list to reverse.
    :type values: list[T]
    :return: A new list with elements in reverse order.
    :rtype: list[T]
    """
    return list(reversed(values))


@worker.task()
def tkr_extend[T: PType](first: list[T], second: list[T]) -> list[T]:
    """Extend a list with elements from another list.

    :param first: The list to extend.
    :type first: list[T]
    :param second: The list of elements to add.
    :type second: list[T]
    :return: The extended list.
    :rtype: list[T]
    """
    first.extend(second)
    return first


@worker.task()
def concat_lists[U: PType, V: PType](first: list[U], second: list[V]) -> list[U | V]:
    """Concatenate two lists of potentially different types.

    :param first: The first list.
    :type first: list[U]
    :param second: The second list.
    :type second: list[V]
    :return: A concatenated list containing elements from both lists.
    :rtype: list[U | V]
    """
    return first + second


@worker.task()
def tkr_str(*, value: float | bool) -> str:
    """Convert a float or bool value to a string.

    :param value: The value to convert.
    :type value: float | bool
    :return: The string representation of the value.
    :rtype: str
    """
    return str(value)


@worker.task()
def tkr_int(*, value: float | bool | str) -> int:
    """Convert a float, bool, or string value to an integer.

    :param value: The value to convert.
    :type value: float | bool | str
    :return: The integer representation of the value.
    :rtype: int
    """
    return int(value)


@worker.task()
def sum_list(values: list[int | float]) -> int | float:
    """Sum all elements in a list of numbers.

    :param values: The list of numeric values.
    :type values: list[int | float]
    :return: The sum of all elements.
    :rtype: int | float
    """
    return sum(values)


@worker.task()
def prod_list(values: list[int | float]) -> int | float:
    """Calculate the product of all elements in a list of numbers.

    :param values: The list of numeric values.
    :type values: list[int | float]
    :return: The product of all elements.
    :rtype: int | float
    """
    prod = 1
    for v in values:
        prod *= v
    return prod


@worker.task()
def max_item(values: list[int | float]) -> int | float:
    """Return the maximum element from a list of numbers.

    :param values: The list of numeric values.
    :type values: list[int | float]
    :return: The maximum value in the list.
    :rtype: int | float
    """
    return max(values)


@worker.task()
def min_item(values: list[int | float]) -> int | float:
    """Return the minimum element from a list of numbers.

    :param values: The list of numeric values.
    :type values: list[int | float]
    :return: The minimum value in the list.
    :rtype: int | float
    """
    return min(values)


@worker.task()
def sort_number_list(values: list[int | float]) -> list[int | float]:
    """Sort a list of numbers in ascending order.

    :param values: The list of numeric values.
    :type values: list[int | float]
    :return: A sorted list of numeric values.
    :rtype: list[int | float]
    """
    return sorted(values)


@worker.task()
def sort_string_list(values: list[str]) -> list[str]:
    """Sort a list of strings in ascending order.

    :param values: The list of strings.
    :type values: list[str]
    :return: A sorted list of strings.
    :rtype: list[str]
    """
    return sorted(values)


@worker.task()
def flatten[T: PType](values: list[list[T]]) -> list[T]:
    """Flatten a list of lists into a single list.

    :param values: The list of lists to flatten.
    :type values: list[list[T]]
    :return: A flattened list containing all elements.
    :rtype: list[T]
    """
    out = []
    for sub in values:
        out.extend(sub)
    return out


@worker.task()
def take[T: PType](values: list[T], n: int) -> list[T]:
    """Return the first n elements of a list.

    :param values: The list.
    :type values: list[T]
    :param n: The number of elements to take.
    :type n: int
    :return: A list containing the first n elements.
    :rtype: list[T]
    """
    return values[:n]


@worker.task()
def drop[T: PType](values: list[T], n: int) -> list[T]:
    """Drop the first n elements of a list and return the rest.

    :param values: The list.
    :type values: list[T]
    :param n: The number of elements to drop.
    :type n: int
    :return: A list with the first n elements removed.
    :rtype: list[T]
    """
    return values[n:]


if __name__ == "__main__":
    worker.app(argv)
