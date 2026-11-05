@portmapping
model Result {
  @file("carryover.bin") carryover: bytes;
  energy: float;
}

interface shell_worker {
  diagonalize(
    @file("fcidump.txt") fcidump: bytes,
    block: integer,
    @args flags: Array<string>
  ): Result;
}
