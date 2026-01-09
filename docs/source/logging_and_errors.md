# Logging and Errors

## Logging

Tierkreis stores a central log of the execution of a graph.
The log is located at `<checkpoint_dir>/<workflow_id>/logs`.
By default, the log level is set to `INFO`.
To change the log level you can get the tierkreis logger:

```python
import logging

logger = logging.getLogger("tierkreis")
logger.setLevel(...)
```

### Workers

Similarly in a worker you can access the logger or overwrite it with your own.

```python
worker.logger.setLevel(...)
worker.setLogger(...)
```

When running a python worker, they will check the environment variable `$TKR_LOG_LEVEL`, `$TKR_LOG_FORMAT` and `$TKR_DATE_FORMAT` and add a streaming handler to the root logger.
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
from tierkreis.builder import GraphBuilder
from tierkreis.controller.data.models import TKR

g = GraphBuilder(TKR[int], TKR[list[int]])
g.outputs(g.inputs)
```

will provide an error message as an integer cant be converted to a list:

```
Argument of type "TKR[int]" cannot be assigned to parameter "outputs" of type "TKR[list[int]]" in function "outputs"
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

In both cases, the controller will stop the execution, raising a `TierkreisError`.

#### Accessing error information

Error information is available in two places.
When running [`run_graph`](#tierkreis.controller.run_graph) or [`run_workflow`](#tierkreis.cli.run_workflow.run_workflow), error information including a stack trace will be printed to `stdout`.
For example running the example `error_handling_graph.py` will produce the following output

```
Graph finished with errors.

<stack_trace for node N0>

Node: -.N0 encountered an error
Stderr information is available at <checkpoints_dir>/<workflow_id>/-.N0/errors
I refuse!

--- Tierkreis graph errors above this line. ---
```

This tells us the following information:

1. A list of nodes that have encountered an error in this case `-.NO` from `Node: -.N0 encountered an error`
2. Where to find error information, typically in a fille called `errors` in the node: `Stderr information is available at <checkpoints_dir>/<workflow_id>/-.N0/errors`
3. The reason for the error: `I refuse!` typically this would include a stack trace, but here we just raised a simple python `Exception`

Further information can be found in the workflows log file, typically located at `<checkpoints_dir>/<workflow_id>/logs`.

#### Configuring Logs

By default tierkreis will not show any logging statements as it just writes a log file instead.
If you want you can provide your own configuration.

```py
logging.basicConfig(
    format="%(asctime)s: %(message)s",
    datefmt="%Y-%m-%dT%H:%M:%S%z",
    level=logging.INFO,
)
```

For example by uncommenting lines 24-28 in the example.

#### Resume a Workflow

Currently, workflows will immediately terminate on an error.
Tasks running in the background might still complete.
Once the error has been analyzed and resolved, you should delete the temporary data e.g. in the example above `rm -rf <checkpoints_dir>/<workflow_id>/-.N0/ ` or use the [`clean_graph_files`](#tierkreis.controller.storage.protocol.ControllerStorage.clean_graph_files) function before rerunning the graph.

## Visualizer

If you're using the visualize to debug workflow, error information will be immediately visible to you.
On the landing page, the workflows table will show you all nodes containing an error.
In then workflow view a red node indicates an error.
Logging information is available by double clicking a node; error information by pressing the "!" button on errored nodes.
For a guide to the visualize please refer to [this document](./visualization.md)
