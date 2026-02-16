"""Code generated from graph namespace. Please do not edit."""

from typing import Literal, NamedTuple, Sequence, TypeVar, Generic, Protocol, Union
from types import NoneType
from tierkreis.controller.data.models import TKR, OpaqueType
from tierkreis.controller.data.types import PType, Struct



class doubler_plus_graph(NamedTuple):
    

    @staticmethod
    def out() -> type[TKR[OpaqueType["tierkreis.controller.data.graph.GraphData"]]]: # noqa: F821 # fmt: skip
        return TKR[OpaqueType["tierkreis.controller.data.graph.GraphData"]] # noqa: F821 # fmt: skip

    @property
    def namespace(self) -> str:
        return "graph" 
