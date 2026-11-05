# Task interfaces: TypeSpec and CWL

These three self-contained examples each include both interface definitions and
a handwritten `jq` worker. They demonstrate Python stub generation, not CWL
execution or workflows. Neither format generates shell code.

## Generate both APIs

From the repository root:

```sh
uv run --all-extras python docs/source/examples/cwl_tasks/generate_stubs.py
```

[generate_stubs.py](generate_stubs.py) writes these persistent files:

```text
generated/
  openssl/
    tsp_api.py
    cwl_api.py
  cat/
    tsp_api.py
    cwl_api.py
  text/
    tsp_api.py
    cwl_api.py
```

Use `--output-directory PATH` to select another destination. The script only
generates stubs: it does not invoke tools, generate keys, or compare results.
Generation requires Tierkreis; executing the wrappers additionally requires
Bash, `jq`, and the corresponding command-line tool.

Both formats produce the same task inputs, output port types, and namespace.
TSP names its output models explicitly; CWL uses `<task>_outputs`.
`File` maps to Tierkreis raw `bytes`, not a CWL File object or a path.

For CLI discovery, use either format in a worker's implementation directory
and run `tkr init stubs`. Do not mix TSP and CWL within that directory.
The generation script loads both explicitly so they can coexist here.

## OpenSSL: two inputs and two outputs

All files for this example are together:

- [schema.tsp](openssl/schema.tsp)
- [genrsa.cwl](openssl/genrsa.cwl)
- [main.sh](openssl/main.sh)

The TSP interface is:

```text
@portmapping
model Outputs {
    private_key: bytes;
    public_key: bytes;
}

interface openssl_worker {
    genrsa(numbits: int, passphrase: bytes): Outputs;
}
```

Its CWL counterpart declares:

```yaml
cwlVersion: v1.2
class: CommandLineTool
id: genrsa
inputs:
  numbits: int
  passphrase: File
outputs:
  private_key: File
  public_key: File
```

Both APIs expose `genrsa(numbits: TKR[int], passphrase: TKR[bytes])` in
namespace `openssl_worker`, with `private_key` and `public_key` byte outputs.

The handwritten wrapper follows the existing OpenSSL tutorial. It reads
**paths** from the Tierkreis call document:

```bash
input_numbits=$(jq -er '.inputs.numbits' "$worker_call_args")
input_passphrase=$(jq -er '.inputs.passphrase' "$worker_call_args")
output_private_key=$(jq -er '.outputs.private_key' "$worker_call_args")
output_public_key=$(jq -er '.outputs.public_key' "$worker_call_args")
```

It then reads the integer from its input file and passes the raw passphrase
file to OpenSSL:

```bash
numbits=$(<"$input_numbits")
openssl genrsa -out "$output_private_key" -aes128 \
    -passout "file:$input_passphrase" "$numbits"
openssl rsa -in "$output_private_key" -passin "file:$input_passphrase" \
    -pubout -out "$output_public_key"
```

This CWL definition is an interface for the wrapper, not a standalone execution
recipe for the two OpenSSL commands.

## cat: `@file` and `@args`

All files for the second example are together:

- [schema.tsp](cat/schema.tsp)
- [concatenate.cwl](cat/concatenate.cwl)
- [main.sh](cat/main.sh)

The TSP definition marks a fixed-name input, a fixed-name output, and a list
of command-line arguments:

```text
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
```

The CWL definition uses `File`, a string array with `inputBinding`, and an
output binding:

```yaml
baseCommand: cat
inputs:
  source:
    type: File
    inputBinding:
      position: 2
  options:
    type: string[]
    inputBinding:
      position: 1
stdout: output.txt
outputs:
  result:
    type: File
    outputBinding:
      glob: output.txt
```

Both APIs expose `concatenate(source: TKR[bytes], options: TKR[list[str]])`
in namespace `cat_worker`, returning a byte output `result`.
For example, `options = ["-n"]` numbers lines; `options = []` preserves
the source bytes unchanged.

The wrapper obtains the source and option-file paths with `jq`. It validates
and decodes the JSON string array using NUL-delimited values, preserving spaces,
newlines, and shell metacharacters without `eval`. It copies the source into
a temporary directory as `input.txt`, invokes `cat`, then copies `output.txt`
to the requested output path:

```bash
cat "${options[@]+"${options[@]}"}" input.txt >output.txt
```

The wrapper, not stub generation, implements `@file` staging and `@args`
decoding. CWL command fields and bindings are likewise descriptive only in
Tierkreis. The CWL input's staged basename is not part of its `File` type;
`input.txt` is a wrapper implementation detail.

## Text utilities: multiple functions in one worker

The third example bundles two commands into the same `text_worker` namespace:

- [schema.tsp](text/schema.tsp) declares both functions in one interface.
- [sort_lines.cwl](text/sort_lines.cwl) describes the sorting task.
- [count_lines.cwl](text/count_lines.cwl) describes the line-counting task.
- [main.sh](text/main.sh) dispatches both tasks in one handwritten worker.

```text
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
```

CWL defines one `CommandLineTool` per task, so the two CWL files are loaded
together rather than introducing a workflow or packed document:

```python
namespace = Namespace.from_cwl_files(
    sorted(Path("docs/source/examples/cwl_tasks/text").glob("*.cwl")),
    name="text_worker",
)
namespace.write_stubs(Path("generated/text/cwl_api.py"))
```

The generation script does this automatically. Both generated APIs contain
`sort_lines(source: TKR[bytes])` and `count_lines(source: TKR[bytes])`, each
with namespace `text_worker`. Sorting returns `result: TKR[bytes]`; counting
returns `count: TKR[int]`.

The wrapper reads `.function_name` with `jq` and dispatches using a Bash
`case`. `sort_lines` invokes `sort` with `LC_ALL=C` for deterministic ordering
and writes raw bytes. `count_lines` invokes `wc -l` and uses `jq` to write the
count as a JSON integer. As with `wc -l`, it counts newline characters, not an
unterminated final line. Its CWL file describes that wrapper interface, not a
standalone CWL recipe for converting command output to an integer.

Both functions use the same executable registration:

```toml
[executors.subprocess.worker_commands]
text_worker = ["bash", "/absolute/path/to/cwl_tasks/text/main.sh"]
```

## Worker invocation

All wrappers accept one argument: the absolute path of a
[`WorkerCallArgs`](../../worker/external_workers.md#parsing-inputs-and-outputs)
JSON document. Its `inputs` and `outputs` map port names to absolute file
paths. Scalars and arrays are JSON files; top-level bytes are raw files.
The wrappers check the task name, touch `done_path` on success, and write an
error to `error_path` on failure.

For example, register the cat wrapper with an existing executor:

```toml
[executors.subprocess.worker_commands]
cat_worker = ["bash", "/absolute/path/to/cwl_tasks/cat/main.sh"]
```

The same wrapper works with either generated Python API.
