# Graphs

Computational graphs are the underlying data model for Tierkreis.
Atomic operations are carried out on the graphs *nodes* and *edges* carry values.
The graphs are directed, which means, the order of operations is defined by the edges.
A node can only be evaluated when all its values (incoming edges) are provided.
Hence all parent nodes need to be evaluated beforehand.
Using this information, Tierkreis finds all nodes for computation by starting from the outputs.
From this follows an important distinction: In order to run a graph, its inputs and outputs need to be defined.
The resulting object is called a `Workflow`.


```{toctree}
:hidden:
:maxdepth: 1
builtins.md
inputs.md
eval.md
loop.md
map.md
```

## Types of nodes

This tutorial will guide you through using the basic nodes for constructing a graph.
The available node types are:
- Input
- Output
- [const](#tierkreis.builder.Graph.const): constant values
- [ifelse](#tierkreis.builder.Graph.ifelse): **lazy** evaluated conditional statement with two branches
- [eifelse](#tierkreis.builder.Graph.eifelse): **eager** evaluated conditional statement with two branches
- [task](#tierkreis.builder.Graph.task): worker tasks, shell scripts etc...
- [eval](#tierkreis.builder.Graph.eval): *higher-order* graph evaluation; evaluating a subgraph
- [loop](#tierkreis.builder.Graph.loop): *higher-order* loop; iterate over a graph until a boolean termination criterion is met
- [map](#tierkreis.builder.Graph.map): *higher-order* map; evaluate the same graph over a collection of independent inputs
- [embed](#tierkreis.builder.Graph.embed): *higher-order* graph embedding; embeds a graph into another graph inplace. 

```{note}
Higher-order in this context means that the node expects one of the inputs to be a graph in the form of `Workflow`s  where inputs and and ouptus are defined.
`Task` nodes can consume *and produce* `Workflow`s.
```


## Node Metadata

Each node can be annotated with metadata. Currently this is only used to enable breakpoints.
