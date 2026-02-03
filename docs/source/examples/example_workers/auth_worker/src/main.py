import logging
import secrets
from sys import argv
from time import time
from typing import NamedTuple, cast

from tierkreis.controller.data.models import portmapping
import pyscrypt  # type: ignore
from cryptography.exceptions import InvalidSignature
from cryptography.hazmat.primitives import serialization
from cryptography.hazmat.primitives.hashes import SHA256
from cryptography.hazmat.primitives.asymmetric import padding
from cryptography.hazmat.primitives.asymmetric.rsa import RSAPrivateKey, RSAPublicKey
from tierkreis import Worker

worker = Worker("auth_worker")
logger = logging.getLogger(__name__)


@portmapping
class EncryptionResult(NamedTuple):
    ciphertext: str
    time_taken: float


@portmapping
class SigningResult(NamedTuple):
    hex_signature: str
    time_taken: float


@worker.task()
def encrypt(plaintext: str, work_factor: int) -> EncryptionResult:
    start_time = time()
    salt = secrets.token_bytes(32)
    ciphertext = pyscrypt.hash(  # type:ignore
        password=plaintext.encode(), salt=salt, N=work_factor, r=1, p=1, dkLen=32
    )
    time_taken = time() - start_time

    return EncryptionResult(ciphertext=str(ciphertext), time_taken=time_taken)


@worker.task()
def sign(private_key: bytes, passphrase: bytes, message: str) -> SigningResult:
    start_time = time()
    key = cast(
        RSAPrivateKey,
        serialization.load_pem_private_key(private_key, password=passphrase),
    )
    signature = key.sign(
        message.encode(),
        padding=padding.PSS(
            mgf=padding.MGF1(SHA256()), salt_length=padding.PSS.MAX_LENGTH
        ),
        algorithm=SHA256(),
    ).hex()
    time_taken = time() - start_time

    return SigningResult(signature, time_taken)


@worker.task()
def verify(public_key: bytes, signature: str, message: str) -> bool:
    key = cast(RSAPublicKey, serialization.load_pem_public_key(public_key))
    try:
        key.verify(
            bytes.fromhex(signature),
            message.encode(),
            padding=padding.PSS(
                mgf=padding.MGF1(SHA256()), salt_length=padding.PSS.MAX_LENGTH
            ),
            algorithm=SHA256(),
        )
        return True
    except InvalidSignature:
        return False


if __name__ == "__main__":
    worker.app(argv)
