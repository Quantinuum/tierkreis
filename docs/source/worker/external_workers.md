# External Workers

Tierkreis does not put any restrictions on workers except the general [contract](../executors/index.md).
This means that you can also write workers in your preferred language and interface it with Tierkreis.
To incorporate the worker into a Tierkreis graph, describe its tasks using a
subset of [TypeSpec](https://typespec.io) or
[CWL v1.2 CommandLineTool](https://www.commonwl.org/v1.2/CommandLineTool.html)
to generate typed stubs, or use the untyped functions in the graph builder.
To run a command-line worker, configure it in the
[`SubprocessExecutor`](../executors/shell.md). The conventional command name is
discovered automatically, or `worker_commands` can select an arbitrary binary
or script. Write a custom executor only when the subprocess protocol is not
sufficient.

For a complete shell-worker example, see the
[`signing_graph`](../examples/signing_graph.ipynb) tutorial.

## IDL

A TypeSpec file can be used to define the inputs and outputs of the tasks in your workers.  
Tierkreis is restricted to a subset of the full specification:

- Interfaces: defining the available tasks in your worker. Currently, Tierkreis allows only **one** interface definition for a worker, which **must** have the same name as the directory it is placed in.
- Models: to define the types of the task in- and outputs
  Additionally, you may use the `@portmapping` decorator on models to indicate tasks with multiple outputs.

For example you could define the following interface in `TestNamespace/namespace.tsp`

```rust
@portmapping
model Person {
  name: string;
  age: uint8;
}

model Dog{
  name: string;
  color: string;
}

@portmapping
model Creature<T> {
  name: string;
  t: T;
}
interface TestNamespace {
  foo(age: integer, name: string): Person;
  bar(): Dog;
  baz<T>(c: Creature<T>): Creature<T>;
}
```

From which you could generate stubs.
This is currently available through a python script or by running the cli `tkr init stubs` with the correct worker directory.

```py
# /// script
# requires-python = ">=3.12"
# dependencies = ["tierkreis"]
# ///

from pathlib import Path
from tierkreis.namespace import Namespace


if __name__ == "__main__":
    tsp_path = Path("<path to your spec file>")
    namespace = Namespace.from_spec_file(tsp_path)
    namespace.write_stubs(tsp_path.parent / "stubs.py")
```

## Parsing Inputs and Outputs

The subprocess executor passes one argument to the invoked command: the path
to a [`WorkerCallArgs`](#tierkreis.data.location.WorkerCallArgs) JSON
document. It contains the function name and the paths of every input and
expected output. The worker reads values from the paths in `inputs` and writes
results to the matching paths in `outputs`. Shell workers can use a JSON tool
such as `jq`; workers in other languages can use their standard JSON parser.

The subprocess and HPC executors also export the call as environment variables,
so scripts do not need a JSON parser:

| Variable | Value |
|---|---|
| `TKR_FUNCTION_NAME` | The task to run. |
| `TKR_INPUT_<port>` | Path of the input value for `<port>`. |
| `TKR_OUTPUT_<port>` | Path to write the output `<port>` to; only set if a downstream node consumes it. |

Port names keep their case; other characters than ASCII letters, digits and `_`
are replaced by `_`. A non-zero exit code marks the task as failed.

### Handwritten shell workers

Stub generation writes only the typed Python API, not a Bash library.
Shell workers implement the worker protocol themselves, for example by
parsing `WorkerCallArgs` with `jq` as in the
[OpenSSL example](../examples/signing_graph.ipynb).

TypeSpec still accepts `@file("<name>")` on `bytes` ports and `@args` on
`Array<string>` inputs as interface metadata. These annotations do not stage
files or decode arguments automatically. The wrapper is responsible for
reading input values, preparing any fixed filenames, invoking the tool, and
writing requested outputs to their storage paths.

### Errors and Logging

`WorkerCallArgs` also supplies `done_path`, `error_path`, and an optional
`logs_path`. A protocol-compatible worker touches `done_path` after writing all
outputs and writes a useful message to `error_path` when it fails. A non-zero
exit status also tells the executor that the task failed.

## CWL task definitions

CWL is an alternative **interface-definition frontend**. Tierkreis extracts
task input/output types and generates Python stubs; it does not generate
shell code, execute CWL commands, or load CWL workflows. YAML parsing uses
PyYAML, with no CWL-library or runner dependency.

For self-contained OpenSSL, `cat`, and multi-function text-worker examples with
both definitions, a stub generation script, and handwritten `jq` wrappers, see
[Comparing TypeSpec and CWL tasks](../examples/cwl_tasks/index.md).

Place one `.cwl` file per task under the worker's implementation directory:

```text
workers/
  openssl_worker/
    tkr_openssl_worker_impl/
      genrsa.cwl
```

Run `tkr init stubs --worker-directory workers` to generate `api/api.py`.
Multiple CWL files are combined into the worker directory's
namespace. A task's local `id` determines its function name, defaulting to
the file stem. Python callers can use
`Namespace.from_cwl_files(paths, name="openssl_worker")`, or
`Namespace.from_spec_file(path)` for a single task with the file stem as its
namespace.

Do not mix TSP and CWL definitions within one worker directory. Existing TSP
and Python workers remain supported. Register the **handwritten** wrapper
using `worker_commands`, exactly as for a TSP-defined worker:

```toml
[executors.subprocess.worker_commands]
openssl_worker = ["bash", "/absolute/path/to/tkr_openssl_worker_impl/main.sh"]
```

### Supported interface subset

- YAML or JSON-form CWL v1.2 `CommandLineTool`, with map or list declarations.
  Task and port names must also be valid Python identifiers and must not clash
  with generated API symbols. Port names cannot start with `_`.
- Concrete `boolean`, `int`, `long`, `float`, `double`, `string`, `File`, and
  arrays using `type[]` or `{type: array, items: ...}`.
  `File` maps to raw `bytes`; arrays map to Python lists.
  Outputs are named Tierkreis portmappings.
- `label` and `doc` metadata.
- Command fields (`baseCommand`, `arguments`, `stdin`, `stdout`, `stderr`)
  and port binding objects may be present but are **not interpreted or
  validated for execution**. They have no effect on the generated API.

Workers continue to parse Tierkreis `WorkerCallArgs` and read/write its port
paths. A CWL `File` declaration does not change those values into CWL File
objects, and no `cwl.output.json` is read.

Unsupported interface syntax fails explicitly, including workflows, expression
tools, packed documents, imports, requirements, directories, defaults, nullable
unions, records, enums, and generic types. This is not a full CWL validator or
implementation; use TSP for interfaces outside the supported subset.

## C++ Example

This example shows how to parse the following interface:

```rust

interface MyWorker {
    double(value: Array<integer>): Array<integer>;
}
```

using [nlohmann/json](https://github.com/nlohmann/json).

```cpp
void parse_json(const std::string &call_args_path, json &call_args)
{
  std::ifstream call_args_file(call_args_path);
  if (!call_args_file.is_open()) // check if open is necessary for << syntax
  {
    std::cerr << "Error: Could not open configuration file: " << call_args_path << std::endl;
    MPI_Abort(MPI_COMM_WORLD, 1);
    return;
  }
  try
  {
    call_args = json::parse(call_args_file);
  }
  catch (json::parse_error &e)
  {
    std::cerr << "Error: JSON parsing failed: " << e.what() << std::endl;
    MPI_Abort(MPI_COMM_WORLD, 1);
    return;
  }
  call_args_file.close();
}

void parse_input(const json &call_args, std::vector<int> &input_data)
{
  json inputs = call_args["inputs"];
  std::cout << inputs << std::endl;
  for (auto &[key, path] : inputs.items())
  {
    if (key != "value")
      continue; // we only want to load value

    auto abs_path = CHECKPOINTS_DIRECTORY + path.template get<std::string>();
    std::cout << "parsing input: " << key << " at: " << abs_path << std::endl;
    json data;
    parse_json(abs_path, data);
    std::cout << "Read data" << data << std::endl;
    auto tmp = data.template get<std::string>();
    std::vector<int> ret = json::parse(tmp);
    input_data.resize(ret.size());
    std::copy(ret.begin(), ret.end(), input_data.begin());
  }
}

void write_output(const json &call_args, const std::vector<int> &gathered_data)
{

  json outputs = call_args["outputs"];
  for (auto &[key, path] : outputs.items())
  {
    if (key != "value")
      continue; // we only want to write value
    std::cout << "writing output: " << key << " at: " << path << std::endl;
    auto out_path = CHECKPOINTS_DIRECTORY + path.template get<std::string>();
    std::ofstream output_file(out_path);
    if (!output_file.is_open())
    {
      std::cerr << "Error: Could not open output file for writing: " << out_path << std::endl;
      MPI_Abort(MPI_COMM_WORLD, 1);
      return;
    }
    json data = gathered_data;
    output_file << data << std::endl;
    output_file.close();
  }

  auto done_file_path = CHECKPOINTS_DIRECTORY + call_args["done_path"].template get<std::string>();
  std::ofstream done_file(done_file_path);
  if (done_file.is_open())
  {
    done_file.close();
  }
  else
  {
    std::cerr << "Error: Could not create done file at " << done_file_path << std::endl;
  }
}
```
