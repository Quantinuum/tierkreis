"""Code generated from error_worker namespace. Please do not edit."""

from typing import Literal, NamedTuple, Sequence, TypeVar, Generic, Protocol, Union
from types import NoneType
from tierkreis.controller.data.models import TKR, OpaqueType
from tierkreis.controller.data.types import PType, Struct



class fail(NamedTuple):
    

    @staticmethod
    def out() -> type[TKR[str]]: # noqa: F821 # fmt: skip
        return TKR[str] # noqa: F821 # fmt: skip

    @property
    def namespace(self) -> str:
        return "error_worker" 
