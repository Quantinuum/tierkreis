"""Code generated from error_worker namespace. Please do not edit."""

# ruff: noqa: F821
from typing import NamedTuple
from tierkreis.controller.data.models import TKR


class fail(NamedTuple):
    @staticmethod
    def out() -> type[TKR[str]]:
        return TKR[str]

    @property
    def namespace(self) -> str:
        return "error_worker"
