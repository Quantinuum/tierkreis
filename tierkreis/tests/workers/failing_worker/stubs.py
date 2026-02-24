"""Code generated from failing_worker namespace. Please do not edit."""

from typing import NamedTuple
from tierkreis.controller.data.models import TKR


class fail(NamedTuple):
    @staticmethod
    def out() -> type[TKR[int]]:  # noqa: F821 # fmt: skip
        return TKR[int]  # noqa: F821 # fmt: skip

    @property
    def namespace(self) -> str:
        return "failing_worker"


class wont_fail(NamedTuple):
    @staticmethod
    def out() -> type[TKR[int]]:  # noqa: F821 # fmt: skip
        return TKR[int]  # noqa: F821 # fmt: skip

    @property
    def namespace(self) -> str:
        return "failing_worker"


class exit_code_1(NamedTuple):
    @staticmethod
    def out() -> type[TKR[int]]:  # noqa: F821 # fmt: skip
        return TKR[int]  # noqa: F821 # fmt: skip

    @property
    def namespace(self) -> str:
        return "failing_worker"
