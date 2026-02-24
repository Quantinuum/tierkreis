"""Elementary parser and parser combinators.

Similar to https://github.com/drhagen/parsita
But https://github.com/drhagen/parsita/blob/83925f035d0777debfe5a6cb53b4944b4b5bcfe2/src/parsita/parsers/_sequential.py#L10
doesn't type check things correctly.
"""

from __future__ import annotations

import contextlib
import re
from typing import TYPE_CHECKING, Any, Never, overload

if TYPE_CHECKING:
    from collections.abc import Callable
from tierkreis.exceptions import TierkreisError


class ParserError(TierkreisError):
    """An error raised when parsing fails in Tierkreis."""


class Parser[T]:
    """A parser for an arbitrary type in tierkreis.

    :fields:
        fn: The parsing function.
    """

    fn: Callable[[str], tuple[T, str]]

    def __init__(self, fn: Callable[[str], tuple[T, str]]) -> None:
        self.fn = fn

    def __call__(self, ins: str) -> tuple[T, str]:
        """Call the parses on a string.

        :param ins: The string to parse.
        :type ins: str
        :return: The parsed string and its type.
        :rtype: tuple[T, str]
        """
        ins = ins.strip()
        return self.fn(ins)

    def __or__[S](
        self,
        other: Parser[S] | Callable[[str], tuple[S, str]],
    ) -> Parser[T | S]:
        """Try the left parser and only if it fails try the right parser."""

        def f(ins: str) -> tuple[T, str] | tuple[S, str]:
            try:
                return self(ins)
            except ParserError:
                return other(ins)

        return Parser(f)

    def __and__[S](
        self,
        other: Parser[S] | Callable[[str], tuple[S, str]],
    ) -> Parser[tuple[T, S]]:
        """Use the left parser and then use the right parser on the remaining input."""

        def f(ins: str) -> tuple[tuple[T, S], str]:
            s, remaining = self(ins)
            t, remaining = other(remaining)
            return (s, t), remaining

        return Parser(f)

    def __lshift__[S](
        self,
        other: Parser[S] | Callable[[str], tuple[S, str]],
    ) -> Parser[T]:
        """Leftshift parsers.

        Use the left parser and then the right parser
        but discard the result of the right parser.
        """

        def f(ins: str) -> tuple[T, str]:
            t, remaining = self(ins)
            _, remaining = other(remaining)
            return t, remaining

        return Parser(f)

    def __rshift__[S](
        self,
        other: Parser[S] | Callable[[str], tuple[S, str]],
    ) -> Parser[S]:
        """Rightshift parsers.

        Use the left parser and then the right parser
        but discard the result of the left parser.
        """

        def f(ins: str) -> tuple[S, str]:
            _, remaining = self(ins)
            s, remaining = other(remaining)
            return s, remaining

        return Parser(f)

    def opt(self) -> Parser[T | None]:
        """Make the parser optional; if it fails then return None and carry on."""

        def f(ins: str) -> tuple[T, str] | tuple[None, str]:
            try:
                return self(ins)
            except ParserError:
                return None, ins

        return Parser(f)

    def map[A](self, fn: Callable[[T], A]) -> Parser[A]:
        """Apply `fn` to transform the output of the parser."""

        def f(ins: str) -> tuple[A, str]:
            t, remaining = self(ins)
            return fn(t), remaining

        return Parser(f)

    def coerce[A](self, a: A) -> Parser[A]:
        """Shorthand for maps that don't need an argument.

        Not strictly speaking required.
        """

        def f(ins: str) -> tuple[A, str]:
            _t, remaining = self(ins)
            return a, remaining

        return Parser(f)

    def rep(self, sep: Parser[str] | None = None) -> Parser[list[T]]:
        """Repeatedly apply a parser with an optional separator.

        The results of the separator parser are discarded.
        """

        def f(ins: str) -> tuple[list[T], str]:
            outs: list[T] = []
            while True:
                try:
                    t, ins = self(ins)
                    if sep:
                        with contextlib.suppress(ParserError):
                            _, ins = sep(ins)
                    outs.append(t)
                except ParserError:
                    break
            return outs, ins

        return Parser(f)

    def fail(self, entity: str) -> Parser[Never]:
        """Fail early if we find something we don't support.

        Not strictly speaking required.
        """

        def f(ins: str) -> Never:
            self(ins)
            msg = f"{entity} not supported."
            raise TierkreisError(msg)

        return Parser(f)


@overload
def seq[A, B](*args: *tuple[Parser[A], Parser[B]]) -> Parser[tuple[A, B]]: ...
@overload
def seq[A, B, C](
    *args: *tuple[Parser[A], Parser[B], Parser[C]],
) -> Parser[tuple[A, B, C]]: ...
@overload
def seq[A, B, C, D](
    *args: *tuple[Parser[A], Parser[B], Parser[C], Parser[D]],
) -> Parser[tuple[A, B, C, D]]: ...
@overload
def seq[A, B, C, D, E](
    *args: *tuple[Parser[A], Parser[B], Parser[C], Parser[D], Parser[E]],
) -> Parser[tuple[A, B, C, D, E]]: ...
def seq(*args: Parser) -> Parser[tuple]:
    """Run a sequence of parsers.

    Runs parsers one after the other and collect their outputs in a tuple.
    """

    def f(ins: str) -> tuple[tuple[Any, ...], str]:
        outs = []
        for arg in args:
            s, ins = arg(ins)
            outs.append(s)
        return tuple(outs), ins

    return Parser(f)


def lit(*args: str) -> Parser[str]:
    """Match literal strings at the start of stream and remove them.

    If the input starts with one of the strings in `args`
    then take the string off the stream and return it.
    """

    def f(ins: str) -> tuple[str, str]:
        for a in args:
            if ins.startswith(a):
                return a, ins[len(a) :]

        msg = f"lit: expected {args} found '{ins[:20]}'"
        raise ParserError(msg)

    return Parser(f)


def reg(regex: str) -> Parser[str]:
    """Match a regex against the start of stream and remove it.

    If start of the input matches the `regex` then take the matching text off
    the stream and return it.
    Please don't pass match groups within the regex; they will be taken care of.
    """

    def f(ins: str) -> tuple[str, str]:
        r = re.compile("^(" + regex + ")")

        if a := r.match(ins):
            return a.group(0), ins[a.end() :]

        msg = f"reg: expected regex {regex} found '{ins[:20]}'"
        raise ParserError(msg)

    return Parser(f)
