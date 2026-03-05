from sys import argv
from typing import NamedTuple

from tierkreis import Worker
from tierkreis.builder import GraphBuilder, TypedGraphRef
from tierkreis.builtins import iadd, itimes
from tierkreis.controller.data.graph import GraphData
from tierkreis.models import TKR

worker = Worker("graph")


@worker.task()
def doubler_plus_graph() -> GraphData:
    g = GraphBuilder(TKR[int], TKR[int])
    double = g.task(itimes(g.inputs, g.const(2)))
    out = g.task(iadd(double, g.const(1)))
    g.outputs(out)
    return g.get_data()


@worker.task()
# The input graph here is expected to be int->int, but we have no way to express that in the type system.
# (GraphBuilder doesn't work as it's not accepted by the stub generator)
def graph_of_graph(f: GraphData, n: int) -> GraphData:
    """Builds a new graph: lambda x: f^n(x)

    I.e. the graph applies the first argument `f` to the graph's input `n` times.
    The graph contains the argument graph `f` as a constant."""
    g = GraphBuilder(TKR[int], TKR[int])
    v = g.inputs
    ref = TypedGraphRef(g.const(f).value_ref(), TKR[int], TKR[int])
    for _ in range(n):
        v = g.eval(ref, v)
    g.outputs(v)
    return g.get_data()


class ApplyTwiceInput(NamedTuple):
    graph: TKR[GraphData]
    value: TKR[int]


@worker.task()
def apply_twice() -> GraphData:
    """Returns a graph for lambda f,x: f(f(x)).

    That is, `f` and `x` are inputs to the graph, not the worker function building it.
    """
    g = GraphBuilder(ApplyTwiceInput, TKR[int])
    f = TypedGraphRef(g.inputs.graph.value_ref(), TKR[int], TKR[int])
    run_once = g.eval(f, g.inputs.value)
    run_twice = g.eval(f, run_once)
    g.outputs(run_twice)
    return g.get_data()


if __name__ == "__main__":
    worker.app(argv)
