@portmapping
model Outputs {
    @file("output.txt") result: bytes;
}

interface cat_worker {
    concatenate(
        @file("input.txt") source: bytes,
        @args options: Array<string>
    ): Outputs;
}
