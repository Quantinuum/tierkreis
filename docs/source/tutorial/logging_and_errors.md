# Logging and Errors

## Logging

Tierkreis stores a central log of the execution of a graph.
The log is located at `<checkpoint_dir>/<workflow_id>/logs`.
By default, the log level is set to `INFO`.

### Workers

When running a python worker, Tierkreis will check the environment variables `$TKR_LOG_LEVEL`, `$TKR_LOG_FORMAT` and `$TKR_DATE_FORMAT` and add a streaming handler to the root logger with the appropriate level and formatter.
You can either set this manually or provided it as part of the `env` argument of an executor.

## Error Handling

Errors can occur at multiple points when writing and running a Tierkreis graph.
Conceptually, there are two classes of errors; We will discuss them in this document.

- _Static Errors_ occur during construction of the graph.
- _Runtime Errors_ occur during execution of the graph.

### Static Errors

Static errors are a common concept in statically typed programming languages.
In Tierkreis, they occur when a user provides incorrect inputs outputs to nodes in a computation graph.
They **do not** prevent the program from running, but indicate that the computation likely wont complete.
These errors will be indicated by the python type checker.
The goal of such errors is to catch errors before potentially running an expensive graph and running into an error.
For example providing an incorrect type to the output

```python
from tierkreis.builder import Graph
from tierkreis.controller.data.models import TKR

g = Graph(TKR[int], TKR[list[int]])
g.finish_with_outputs(g.inputs)
```

will provide an error message as an integer can't be converted to a list:

```
Argument of type "TKR[int]" cannot be assigned to parameter "outputs" of type "TKR[list[int]]" in function "finish_with_outputs"
  "TKR[int]" is not assignable to "TKR[list[int]]"
    Type parameter "T@TKR" is covariant, but "int" is not a subtype of "list[int]"
      "int" is not assignable to "list[int]"
```

Preventing such errors is useful as they could turn into an runtime error, which terminate the graph execution and can therefore be costly.

### Runtime Errors

Runtime errors typically occur when a worker task fails.
The most common types of errors are:

- Unhandled exceptions (python workers)
- Non-zero exit codes (external workers)

In both cases, `Runtime.wait_for()` raises `ValueError` after the run enters an
error state.

#### Accessing error information

The raised exception has the message `Workflow failed`. Related diagnostic
notes identify failed node locations and include worker error details. Selected
node locations can also be inspected after the run:

```python
try:
    await runtime.wait_for(run_id, 0)
except ValueError as error:
    print(error)

states = await runtime.debug_read_node_states(run_id, 0, ["N0"])
print(states["N0"].status)  # Error
```

#### Configuring Logs

Runtime logging is configured when constructing a custom runtime with
`new_from_config()`. The predefined constructors use the default logging
configuration. Pass the complete runtime configuration dictionary, including
its logging settings, to `new_from_config()`.

#### Resume a Workflow

Workflows terminate after an error. Starting a new run of the saved workflow
creates an independent run ID. Restarting selected nodes and resuming an
existing run are not exposed by the Rust runtime binding yet.

## Debug execution information

`debug_read_node_states()` returns the status of requested node locations. The
current binding exposes `Complete`, `Cancelled`, `Error`, `Running`, `Queued`,
and `Scheduled` statuses.

### Breakpoints

Interactive breakpoints are not exposed by the Rust runtime binding yet.

### Debug Mode

```{warning}
The legacy Python controller still contains breakpoint, resume, stored executor
diagnostic, and in-process Python debugging APIs. These do not apply to the
Rust-backed `Runtime` examples.
```



## Visualizer

If you're using the visualize to debug workflow, error information will be immediately visible to you.
On the landing page, the workflows table will show you all nodes containing an error.
In then workflow view a red node indicates an error.
Logging information is available by double clicking a node; error information by pressing the "!" button on errored nodes.
For a guide to the visualize please refer to [this document](../tutorial//visualization.md)
