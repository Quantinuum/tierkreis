@portmapping
model Outputs {
    private_key: bytes;
    public_key: bytes;
}

interface openssl_worker {
    genrsa (
        numbits: int,
        passphrase: bytes
    ): Outputs;
}
