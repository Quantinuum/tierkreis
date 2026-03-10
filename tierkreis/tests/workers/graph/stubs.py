"""Code generated from graph namespace. Please do not edit."""

from typing import Literal, NamedTuple
from tierkreis.controller.data.models import TKR

type GraphData = Literal["tierkreis.controller.data.graph.GraphData"]


class doubler_plus_graph(NamedTuple):
    @staticmethod
    def out() -> type[TKR[GraphData]]:  # noqa: F821 # fmt: skip
        return TKR[GraphData]  # noqa: F821 # fmt: skip

    @property
    def namespace(self) -> str:
        return "graph"


class graph_of_graph(NamedTuple):
    f: TKR[GraphData]  # noqa: F821 # fmt: skip
    n: TKR[int]  # noqa: F821 # fmt: skip

    @staticmethod
    def out() -> type[TKR[GraphData]]:  # noqa: F821 # fmt: skip
        return TKR[GraphData]  # noqa: F821 # fmt: skip

    @property
    def namespace(self) -> str:
        return "graph"


class apply_twice(NamedTuple):
    @staticmethod
    def out() -> type[TKR[GraphData]]:  # noqa: F821 # fmt: skip
        return TKR[GraphData]  # noqa: F821 # fmt: skip

    @property
    def namespace(self) -> str:
        return "graph"
