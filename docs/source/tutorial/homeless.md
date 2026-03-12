### How to use the new project

The project is set up in a way to be convenient for developers already experienced with Tierkreis.
Some of the concepts we will discuss in the [core concepts](./core_concepts.md) and implement in the upcoming pages.

The repository is structure is intended to separate _graphs_, _workers_ and library code.
Graph definitions, like a `hello_world_graph` you will write in this tutorial should go into the `graphs` directory.
The main file contains an example graph, and has set up the storage and executors similar to the ones above.

`Workers` are a way to add custom functionality, which will be executed as Tierkreis tasks.
If you're not familiar with `workers` yet, we will explain them as part of this tutorial.
It is very simple to wrap existing python code to make it available in Tierkreis.
Each worker is a separate entity, your new project will contain one `example_worker`.
If you want to include more they should have a similar structure.


## Advanced Topics

Once you have finished the tutorial you can start writing your own workflows.
If you want to learn more details to fully leverage the power of Tierkreis,
the advanced  user guide available [here](../tutorial_advanced/index.md).
It includes further tutorials, and detailed descsriptions on how to write graphs, workers and executors.

