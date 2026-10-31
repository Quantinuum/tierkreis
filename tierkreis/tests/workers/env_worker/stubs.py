"""Code generated from env_worker namespace. Please do not edit."""

from typing import NamedTuple

from tierkreis.controller.data.models import TKR


class read_env(NamedTuple):
    name: TKR[str]  # fmt: skip

    @staticmethod
    def out() -> type[TKR[str]]:  # fmt: skip
        return TKR[str]  # fmt: skip

    @property
    def namespace(self) -> str:
        return "env_worker"
