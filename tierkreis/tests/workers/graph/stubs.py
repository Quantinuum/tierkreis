"""Code generated from graph namespace. Please do not edit."""

# ruff: noqa: F821
from typing import NamedTuple
from tierkreis.controller.data.models import TKR
from tierkreis.controller.data.types import Workflow


class ApplyTwiceInput(NamedTuple):
    graph: TKR[Workflow[TKR[int], TKR[int]]]
    value: TKR[int]


class doubler_plus_graph(NamedTuple):
    @staticmethod
    def out() -> type[TKR[Workflow[TKR[int], TKR[int]]]]:
        return TKR[Workflow[TKR[int], TKR[int]]]

    @property
    def namespace(self) -> str:
        return "graph"


class graph_of_graph(NamedTuple):
    f: TKR[Workflow[TKR[int], TKR[int]]]
    n: TKR[int]

    @staticmethod
    def out() -> type[TKR[Workflow[TKR[int], TKR[int]]]]:
        return TKR[Workflow[TKR[int], TKR[int]]]

    @property
    def namespace(self) -> str:
        return "graph"


class apply_twice(NamedTuple):
    @staticmethod
    def out() -> type[TKR[Workflow[ApplyTwiceInput, TKR[int]]]]:
        return TKR[Workflow[ApplyTwiceInput, TKR[int]]]

    @property
    def namespace(self) -> str:
        return "graph"
