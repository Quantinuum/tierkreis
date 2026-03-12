```bash
uv run tkr run -o
> value: 1
```
This will use the following default locations:
1. The graph definition is used from `tkr/graphs/main.py:workflow`
2. The inputs are taken from `workflow_inputs.json`
3. `-o` enables the printing of outputs

This is the same as 
```bash
uv run tkr/graphs/main.py
> Value is: 1
```


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
