cwlVersion: v1.2
class: CommandLineTool
id: genrsa
doc: Typed interface of the accompanying jq-based OpenSSL worker.
inputs:
  numbits: int
  passphrase: File
outputs:
  private_key: File
  public_key: File
