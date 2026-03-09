"""Code generated from openssl_worker namespace. Please do not edit."""

from typing import NamedTuple
from tierkreis.controller.data.models import TKR


class Outputs(NamedTuple):
    private_key: TKR[bytes]  # noqa: F821 # fmt: skip
    public_key: TKR[bytes]  # noqa: F821 # fmt: skip


class genrsa(NamedTuple):
    numbits: TKR[int]  # noqa: F821 # fmt: skip
    passphrase: TKR[bytes]  # noqa: F821 # fmt: skip

    @staticmethod
    def out() -> type[Outputs]:  # noqa: F821 # fmt: skip
        return Outputs  # noqa: F821 # fmt: skip

    @property
    def namespace(self) -> str:
        return "openssl_worker"
