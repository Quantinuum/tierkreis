---
file_format: mystnb
kernelspec:
  name: python3
---

# Shell Executors

Shell Executors allow Tierkreis to dispatch almost any unix compatible program in a shell environment.
Currently, there are two ways to run the shell that differ in how you provide the arguments and. how the outputs are read.

## Shell Executor

The [`ShellExecutor`](#tierkreis.controller.executor.shell_executor.ShellExecutor) uses environment variables to handle in and output.
The following controller variables are exported:

- `$checkpoints_directory`
- `$function_name`
- `$done_path`
- `$error_path`
- `$output_dir`
- `$logs_path`

Locations to inputs and outputs are provide in the form of

- `input_<portname>_file`
- `output_<portname>_file`
  optionally values can also be provided by the executor in the form
- `input_<portname>_value`
  If possible this option should be avoided as values can be potentially very large and exceed shell limitations.
  Below is an example to show how to use this to make a script of executable compatible with Tierkreis.

## StdInOut

[`StdInOut`](#tierkreis.controller.executor.stdinout.StdInOut) reads the first node input from `stdin` and writes to the first output to `stdout`.
Internally this uses the unix shell syntax for redirection

```sh
$> /path/to/binary <first_input_file first_output_file>
```

Beyond that, the executor does not provide further inputs, hence the script has to be able to handle the remainder itself.

```{warning}
This currently supports only a single input and output.
The rest will be discarded.
```

## Example

In this example we will use the shell executor to generate a RSA private key to sign a message.
For this we will use `openssl-genrsa`.
Additionally we will use the generated key to sign a message using the auth workers introduced in the [map example](../graphs/map.md).

For the Shell Executor we need to wrap the `genrsa` in a `main.sh` script parsing the inputs and outputs:

```sh
#!/usr/bin/env bash
numbits=$(cat $input_numbits_file)
openssl genrsa -out $output_private_key_file -aes128 -passout "file:$input_passphrase_file" $numbits
openssl rsa -in $output_private_key_file -passin "file:$input_passphrase_file" -pubout -out $output_public_key_file

```

Then we can define the following graph.
Since we don't have stubs for the script (which we could generate by providing an [IDL] file) we use the untyped version using `Graphdata.func`

```{code} ipython3
from tierkreis.models import EmptyModel, TKR
from tierkreis.builder import GraphBuilder
from ..tutorial.auth_stubs import sign

def signing_graph()-> GraphBuilder[EmptyModel, TKR[str]]:
    g = GraphBuilder(EmptyModel, TKR[str])
    # Define the fix inputs to the graph
    message = g.const("dummymessage")
    passphrase = g.const(b"dummypassphrase")

    # Access the script, by calling genrsa inside the script openssl_workers
    # We need to manually map the inputs to the correct variables
    # These we have defined in the wrapper script
    key_pair = g.data.func(
        "openssl_worker.genrsa",
        {"passphrase": passphrase.value_ref(), "numbits": g.const(4096).value_ref()},
    )
    # Similarly we parse the outputs as we have defined them in the script
    private_key: TKR[bytes] = TKR(*key_pair("private_key"))  # unsafe cast
    public_key: TKR[bytes] = TKR(*key_pair("public_key"))  # unsafe cast

    # Finally we use the auth worker to sign the message
    signing_result = g.task(sign(private_key, passphrase, message)).hex_signature
    g.outputs(signing_result)
    return g
```

Running the graph follows all the usual steps.
We only want to use the shell executor to run the script, so we need to provide a default executor.

```{code} ipython3

from pathlib import Path
from uuid import UUID
from tierkreis import run_graph
from tierkreis.executor import MultipleExecutor, UvExecutor, ShellExecutor
from tierkreis.storage import FileStorage, read_outputs

graph = signing_graph()
storage = FileStorage(UUID(int=105), "sign_graph", do_cleanup=True)
# Make sure to provide the correct directory to the workers
registry_path = Path(__file__).parent / "example_workers"
uv = UvExecutor(registry_path, storage.logs_path)
shell = ShellExecutor(registry_path, storage.workflow_dir)
executor = MultipleExecutor(uv, {"shell": shell}, {"openssl_worker": "shell"})
run_graph(storage, executor, graph().data, {})

# Verify the result
out = read_outputs(graph.get_data(), storage)
assert isinstance(out, str)
print(out)
assert len(out) == 1024
```
