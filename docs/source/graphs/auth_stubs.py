"""Code generated from auth_worker namespace. Please do not edit."""

from typing import NamedTuple

from tierkreis.controller.data.models import TKR


class EncryptionResult(NamedTuple):
    ciphertext: TKR[str]  # fmt: skip
    time_taken: TKR[float]  # fmt: skip


class SigningResult(NamedTuple):
    hex_signature: TKR[str]  # fmt: skip
    time_taken: TKR[float]  # fmt: skip


class encrypt(NamedTuple):
    plaintext: TKR[str]  # fmt: skip
    work_factor: TKR[int]  # fmt: skip

    @staticmethod
    def out() -> type[EncryptionResult]:  # fmt: skip
        return EncryptionResult  # fmt: skip

    @property
    def namespace(self) -> str:
        return "auth_worker"


class sign(NamedTuple):
    private_key: TKR[bytes]  # fmt: skip
    passphrase: TKR[bytes]  # fmt: skip
    message: TKR[str]  # fmt: skip

    @staticmethod
    def out() -> type[SigningResult]:  # fmt: skip
        return SigningResult  # fmt: skip

    @property
    def namespace(self) -> str:
        return "auth_worker"


class verify(NamedTuple):
    public_key: TKR[bytes]  # fmt: skip
    signature: TKR[str]  # fmt: skip
    message: TKR[str]  # fmt: skip

    @staticmethod
    def out() -> type[TKR[bool]]:  # fmt: skip
        return TKR[bool]  # fmt: skip

    @property
    def namespace(self) -> str:
        return "auth_worker"
