"""Code generated from graph namespace. Please do not edit."""

from typing import NamedTuple, Protocol
from tierkreis.controller.data.models import TKR
from tierkreis.controller.data.types import Struct, FinishedGraph


class ApplyTwiceInput(Struct, Protocol):
    graph: TKR[FinishedGraph[TKR[int], TKR[int]]]  # noqa: F821 # fmt: skip
    value: TKR[int]  # noqa: F821 # fmt: skip


class doubler_plus_graph(NamedTuple):
    @staticmethod
    def out() -> type[TKR[FinishedGraph[TKR[int], TKR[int]]]]:  # noqa: F821 # fmt: skip
        return TKR[FinishedGraph[TKR[int], TKR[int]]]  # noqa: F821 # fmt: skip

    @property
    def namespace(self) -> str:
        return "graph"


class graph_of_graph(NamedTuple):
    f: TKR[FinishedGraph[TKR[int], TKR[int]]]  # noqa: F821 # fmt: skip
    n: TKR[int]  # noqa: F821 # fmt: skip

    @staticmethod
    def out() -> type[TKR[FinishedGraph[TKR[int], TKR[int]]]]:  # noqa: F821 # fmt: skip
        return TKR[FinishedGraph[TKR[int], TKR[int]]]  # noqa: F821 # fmt: skip

    @property
    def namespace(self) -> str:
        return "graph"


class apply_twice(NamedTuple):
    @staticmethod
    def out() -> type[TKR[FinishedGraph[ApplyTwiceInput, TKR[int]]]]:  # noqa: F821 # fmt: skip
        return TKR[FinishedGraph[ApplyTwiceInput, TKR[int]]]  # noqa: F821 # fmt: skip

    @property
    def namespace(self) -> str:
        return "graph"
