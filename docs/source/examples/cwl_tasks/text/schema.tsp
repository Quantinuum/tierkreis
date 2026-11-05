@portmapping
model SortedOutput {
    result: bytes;
}

@portmapping
model CountOutput {
    count: int;
}

interface text_worker {
    sort_lines(source: bytes): SortedOutput;
    count_lines(source: bytes): CountOutput;
}
