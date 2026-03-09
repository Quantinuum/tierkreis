"""Code generated from auth_worker namespace. Please do not edit."""

from typing import NamedTuple
from tierkreis.controller.data.models import TKR


class EncryptionResult(NamedTuple):
    ciphertext: TKR[str]  # noqa: F821 # fmt: skip
    time_taken: TKR[float]  # noqa: F821 # fmt: skip


class SigningResult(NamedTuple):
    hex_signature: TKR[str]  # noqa: F821 # fmt: skip
    time_taken: TKR[float]  # noqa: F821 # fmt: skip


class encrypt(NamedTuple):
    plaintext: TKR[str]  # noqa: F821 # fmt: skip
    work_factor: TKR[int]  # noqa: F821 # fmt: skip

    @staticmethod
    def out() -> type[EncryptionResult]:  # noqa: F821 # fmt: skip
        return EncryptionResult  # noqa: F821 # fmt: skip

    @property
    def namespace(self) -> str:
        return "auth_worker"


class sign(NamedTuple):
    private_key: TKR[bytes]  # noqa: F821 # fmt: skip
    passphrase: TKR[bytes]  # noqa: F821 # fmt: skip
    message: TKR[str]  # noqa: F821 # fmt: skip

    @staticmethod
    def out() -> type[SigningResult]:  # noqa: F821 # fmt: skip
        return SigningResult  # noqa: F821 # fmt: skip

    @property
    def namespace(self) -> str:
        return "auth_worker"


class verify(NamedTuple):
    public_key: TKR[bytes]  # noqa: F821 # fmt: skip
    signature: TKR[str]  # noqa: F821 # fmt: skip
    message: TKR[str]  # noqa: F821 # fmt: skip

    @staticmethod
    def out() -> type[TKR[bool]]:  # noqa: F821 # fmt: skip
        return TKR[bool]  # noqa: F821 # fmt: skip

    @property
    def namespace(self) -> str:
        return "auth_worker"
