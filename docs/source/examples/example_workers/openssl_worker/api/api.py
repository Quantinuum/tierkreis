"""Code generated from openssl_worker namespace. Please do not edit."""

from typing import NamedTuple

from tierkreis.controller.data.models import TKR


class Outputs(NamedTuple):
    private_key: TKR[bytes]  # fmt: skip
    public_key: TKR[bytes]  # fmt: skip


class genrsa(NamedTuple):
    numbits: TKR[int]  # fmt: skip
    passphrase: TKR[bytes]  # fmt: skip

    @staticmethod
    def out() -> type[Outputs]:  # fmt: skip
        return Outputs  # fmt: skip

    @property
    def namespace(self) -> str:
        return "openssl_worker"
