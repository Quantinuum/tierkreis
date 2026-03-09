"""Code generated from error_worker namespace. Please do not edit."""

from typing import NamedTuple
from tierkreis.controller.data.models import TKR


class fail(NamedTuple):
    @staticmethod
    def out() -> type[TKR[str]]:  # noqa: F821 # fmt: skip
        return TKR[str]  # noqa: F821 # fmt: skip

    @property
    def namespace(self) -> str:
        return "error_worker"
