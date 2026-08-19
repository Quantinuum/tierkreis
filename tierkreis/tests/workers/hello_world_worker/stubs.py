"""Code generated from hello_world_worker namespace. Please do not edit."""

from typing import NamedTuple

from tierkreis.controller.data.models import TKR


class greet(NamedTuple):
    greeting: TKR[str]  # fmt: skip
    subject: TKR[str]  # fmt: skip

    @staticmethod
    def out() -> type[TKR[str]]:  # fmt: skip
        return TKR[str]  # fmt: skip

    @property
    def namespace(self) -> str:
        return "hello_world_worker"
