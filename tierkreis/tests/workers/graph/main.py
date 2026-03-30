from sys import argv
from typing import NamedTuple

from tierkreis import Worker
from tierkreis.builder import Graph, Workflow, TypedGraphRef
from tierkreis.builtins import iadd, itimes
from tierkreis.models import TKR

worker = Worker("graph")


@worker.task()
def doubler_plus_graph() -> Workflow[TKR[int], TKR[int]]:
    g = Graph(TKR[int], TKR[int])
    double = g.task(itimes(g.inputs, g.const(2)))
    out = g.task(iadd(double, g.const(1)))
    return g.finish_with_outputs(out)


@worker.task()
# The input graph here is expected to be int->int, but we have no way to express that in the type system.
# (Graph doesn't work as it's not accepted by the stub generator)
def graph_of_graph(
    f: Workflow[TKR[int], TKR[int]], n: int
) -> Workflow[TKR[int], TKR[int]]:
    """Builds a new graph: lambda x: f^n(x)

    I.e. the graph applies the first argument `f` to the graph's input `n` times.
    The graph contains the argument graph `f` as a constant."""
    g = Graph(TKR[int], TKR[int])
    v = g.inputs
    ref: TypedGraphRef[TKR[int], TKR[int]] = g.graph_const(f)
    for _ in range(n):
        v = g.eval(ref, v)
    return g.finish_with_outputs(v)


class ApplyTwiceInput(NamedTuple):
    graph: TKR[Workflow[TKR[int], TKR[int]]]
    value: TKR[int]


@worker.task()
def apply_twice() -> Workflow[ApplyTwiceInput, TKR[int]]:
    """Returns a graph for lambda f,x: f(f(x)).

    That is, `f` and `x` are inputs to the graph, not the worker function building it.
    """
    g = Graph(ApplyTwiceInput, TKR[int])
    f = TypedGraphRef(g.inputs.graph, TKR[int])
    run_once = g.eval(f, g.inputs.value)
    run_twice = g.eval(f, run_once)
    return g.finish_with_outputs(run_twice)


if __name__ == "__main__":
    worker.app(argv)
