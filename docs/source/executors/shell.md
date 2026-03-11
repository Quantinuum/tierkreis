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
$> /path/to/binary <first_input_file> <first_output_file>
```

Beyond that, the executor does not provide further inputs, hence the script has to be able to handle the remainder itself.

```{warning}
This currently supports only a single input and output.
The rest will be discarded.
```

## Example

There is an example associated to this document.
You can find it [here](../examples/signing_graph.ipynb).
