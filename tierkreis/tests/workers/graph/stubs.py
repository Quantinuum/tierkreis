"""Code generated from graph namespace. Please do not edit."""

from typing import NamedTuple
from tierkreis.controller.data.models import TKR, Workflow


class ApplyTwiceInput(NamedTuple):
    graph: TKR[Workflow[TKR[int], TKR[int]]]  # noqa: F821 # fmt: skip
    value: TKR[int]  # noqa: F821 # fmt: skip


class doubler_plus_graph(NamedTuple):
    @staticmethod
    def out() -> type[TKR[Workflow[TKR[int], TKR[int]]]]:  # noqa: F821 # fmt: skip
        return TKR[Workflow[TKR[int], TKR[int]]]  # noqa: F821 # fmt: skip

    @property
    def namespace(self) -> str:
        return "graph"


class graph_of_graph(NamedTuple):
    f: TKR[Workflow[TKR[int], TKR[int]]]  # noqa: F821 # fmt: skip
    n: TKR[int]  # noqa: F821 # fmt: skip

    @staticmethod
    def out() -> type[TKR[Workflow[TKR[int], TKR[int]]]]:  # noqa: F821 # fmt: skip
        return TKR[Workflow[TKR[int], TKR[int]]]  # noqa: F821 # fmt: skip

    @property
    def namespace(self) -> str:
        return "graph"


class apply_twice(NamedTuple):
    @staticmethod
    def out() -> type[TKR[Workflow[ApplyTwiceInput, TKR[int]]]]:  # noqa: F821 # fmt: skip
        return TKR[Workflow[ApplyTwiceInput, TKR[int]]]  # noqa: F821 # fmt: skip

    @property
    def namespace(self) -> str:
        return "graph"
