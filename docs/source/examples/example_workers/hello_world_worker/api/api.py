"""Code generated from hello_world_worker namespace. Please do not edit."""

# ruff: noqa: F821
from typing import NamedTuple
from tierkreis.controller.data.models import TKR


class greet(NamedTuple):
    greeting: TKR[str]
    subject: TKR[str]

    @staticmethod
    def out() -> type[TKR[str]]:
        return TKR[str]

    @property
    def namespace(self) -> str:
        return "hello_world_worker"
