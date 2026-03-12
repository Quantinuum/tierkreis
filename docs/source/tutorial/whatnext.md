# What Next?

You've reached the end of the tutorial.  You have learned how to
define a graph, fill it with tasks, and run the workflow.  We only
scratched the surface of what you can do with Tierkreis, but you know
enough to begin using it for your own work.

If you want to learn more details to fully leverage the power of
Tierkreis, the advanced user guide available
[here](user_guide.md).  It includes further tutorials,
and detailed descriptions on how to write graphs, workers and
executors.

You might find the follow topics useful next:

* [Defining a worker from a shell command](../worker/index.md#generating-workers-from-the-cli)
* [Using HPC executors](../executors/hpc.md)
* [Working with the source distribution of Tierkreis](tutorials.md#building-the-examples)



## Documents for writing graphs

A sequence of documents that cover the fundamentals of writing complex Tierkreis graphs.

[Using builtin functions](../graphs/builtins.md)

[Defining graph inputs and outputs](../graphs/inputs.md)

[Nested graphs using Eval](../graphs/eval.md)

[Iteration using Loop](../graphs/loop.md)

[Parallel computation using Map](../graphs/map.md)

## Tutorials for writing workers

Documents on writing workers that provide additional tasks. For a general overview look at the [worker documentation](../worker/index.md)

[Complex types in Tierkreis Python workers](../worker/complex_types.md)

[Tierkreis prebuild workers](../worker/native_workers/index.md)

[External workers with an IDL](../worker/external_workers.md)

## Executors

Finally documentation on executors

[A general overview](../executors/index.md)

[Shell Executors](../executors/shell.md)

[Running workers with HPC Executors](../executors/hpc.md)
