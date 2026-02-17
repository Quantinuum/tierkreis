"""Code generated from graph namespace. Please do not edit."""

from typing import NamedTuple
from tierkreis.controller.data.models import TKR, OpaqueType


class doubler_plus_graph(NamedTuple):
    @staticmethod
    def out() -> type[TKR[OpaqueType["tierkreis.controller.data.graph.GraphData"]]]:  # noqa: F821 # fmt: skip
        return TKR[OpaqueType["tierkreis.controller.data.graph.GraphData"]]  # noqa: F821 # fmt: skip

    @property
    def namespace(self) -> str:
        return "graph"
