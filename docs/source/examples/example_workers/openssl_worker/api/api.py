"""Code generated from openssl_worker namespace. Please do not edit."""

from typing import NamedTuple

from tierkreis.controller.data.models import TKR


class Outputs(NamedTuple):
    private_key: TKR[bytes]
    public_key: TKR[bytes]


class genrsa(NamedTuple):
    numbits: TKR[int]
    passphrase: TKR[bytes]

    @staticmethod
    def out() -> type[Outputs]:
        return Outputs

    @property
    def namespace(self) -> str:
        return "openssl_worker"
