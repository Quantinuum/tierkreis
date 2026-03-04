from typing import NamedTuple

from tests.workers.graph.stubs import doubler_plus_graph, graph_of_graph, apply_twice
from tierkreis.builder import GraphBuilder, TypedGraphRef, FinishedGraph
from tierkreis.builtins import (
    conjugate,
    eq,
    iadd,
    idivide,
    igt,
    itimes,
    mod,
    tkr_tuple,
    untuple,
    tkr_str,
)
from tierkreis.controller.data.core import EmptyModel
from tierkreis.controller.data.models import TKR, OpaqueType


class DoublerInput(NamedTuple):
    x: TKR[int]
    intercept: TKR[int]


class DoublerOutput(NamedTuple):
    a: TKR[int]
    value: TKR[int]


def typed_doubler() -> FinishedGraph[TKR[int], TKR[int]]:
    g = GraphBuilder(TKR[int], TKR[int])
    out = g.task(itimes(a=g.const(2), b=g.inputs))
    return g.finish_with_outputs(out)


def typed_doubler_plus_multi() -> FinishedGraph[DoublerInput, DoublerOutput]:
    g = GraphBuilder(DoublerInput, DoublerOutput)
    mul = g.task(itimes(a=g.inputs.x, b=g.const(2)))
    out = g.task(iadd(a=mul, b=g.inputs.intercept))
    return g.finish_with_outputs(DoublerOutput(a=g.inputs.x, value=out))


def typed_doubler_plus() -> FinishedGraph[DoublerInput, TKR[int]]:
    g = GraphBuilder(DoublerInput, TKR[int])
    mul = g.task(itimes(a=g.inputs.x, b=g.const(2)))
    out = g.task(iadd(a=mul, b=g.inputs.intercept))
    return g.finish_with_outputs(out)


class TypedEvalOutputs(NamedTuple):
    typed_eval_output: TKR[int]


def typed_eval() -> FinishedGraph[EmptyModel, TypedEvalOutputs]:
    g = GraphBuilder(EmptyModel, TypedEvalOutputs)
    e = g.eval(typed_doubler_plus(), DoublerInput(x=g.const(6), intercept=g.const(0)))
    return g.finish_with_outputs(TypedEvalOutputs(typed_eval_output=e))


class LoopBodyInput(NamedTuple):
    loop_acc: TKR[int]


class LoopBodyOutput(NamedTuple):
    loop_acc: TKR[int]
    should_continue: TKR[bool]


def loop_body() -> FinishedGraph[LoopBodyInput, LoopBodyOutput]:
    g = GraphBuilder(LoopBodyInput, LoopBodyOutput)
    a_plus = g.task(iadd(a=g.inputs.loop_acc, b=g.const(1)))
    pred = g.task(igt(a=g.const(10), b=a_plus))
    return g.finish_with_outputs(LoopBodyOutput(loop_acc=a_plus, should_continue=pred))


def typed_loop() -> FinishedGraph[EmptyModel, TKR[int]]:
    g = GraphBuilder(EmptyModel, TKR[int])
    loop = g.loop(loop_body(), LoopBodyInput(loop_acc=g.const(6)))
    return g.finish_with_outputs(loop.loop_acc)


def typed_map_simple() -> FinishedGraph[TKR[list[int]], TKR[list[int]]]:
    g = GraphBuilder(TKR[list[int]], TKR[list[int]])
    m = g.map(typed_doubler(), g.inputs)
    return g.finish_with_outputs(m)


def typed_map() -> FinishedGraph[TKR[list[int]], TKR[list[int]]]:
    g = GraphBuilder(TKR[list[int]], TKR[list[int]])
    ins = g.map(lambda n: DoublerInput(x=n, intercept=g.const(6)), g.inputs)
    m = g.map(typed_doubler_plus(), ins)
    return g.finish_with_outputs(m)


def typed_destructuring() -> FinishedGraph[TKR[list[int]], TKR[list[int]]]:
    g = GraphBuilder(TKR[list[int]], TKR[list[int]])
    ins = g.map(lambda n: DoublerInput(x=n, intercept=g.const(6)), g.inputs)
    m = g.map(typed_doubler_plus_multi(), ins)
    mout = g.map(lambda x: x.value, m)
    return g.finish_with_outputs(mout)


def tuple_untuple() -> FinishedGraph[EmptyModel, TKR[int]]:
    g = GraphBuilder(EmptyModel, TKR[int])
    t = g.task(tkr_tuple(g.const(1), g.const(2)))
    ut = g.task(untuple(t))
    return g.finish_with_outputs(g.task(iadd(ut.a, ut.b)))


def factorial() -> FinishedGraph[TKR[int], TKR[int]]:
    g = GraphBuilder(TKR[int], TKR[int])
    pred = g.task(igt(g.inputs, g.const(1)))
    n_minus_one = g.task(iadd(g.const(-1), g.inputs))
    rec = g.eval(g.ref(), n_minus_one)
    out = g.ifelse(pred, g.task(itimes(g.inputs, rec)), g.const(1))
    return g.finish_with_outputs(out)


class GCDInput(NamedTuple):
    a: TKR[int]
    b: TKR[int]


def gcd() -> FinishedGraph[GCDInput, TKR[int]]:
    g = GraphBuilder(GCDInput, TKR[int])

    pred = g.task(igt(g.inputs.b, g.const(0)))
    a_mod_b = g.task(mod(g.inputs.a, g.inputs.b))
    rec = g.eval(g.ref(), GCDInput(a=g.inputs.b, b=a_mod_b))

    return g.finish_with_outputs(g.ifelse(pred, rec, g.inputs.a))


def tkr_conj() -> FinishedGraph[TKR[complex], TKR[complex]]:
    g = GraphBuilder(TKR[complex], TKR[complex])
    z = g.task(conjugate(g.inputs))
    return g.finish_with_outputs(z)


def tkr_list_conj() -> FinishedGraph[TKR[list[complex]], TKR[list[complex]]]:
    g = GraphBuilder(TKR[list[complex]], TKR[list[complex]])
    zs = g.map(tkr_conj(), g.inputs)
    return g.finish_with_outputs(zs)


def eval_body_is_from_worker() -> FinishedGraph[TKR[int], TKR[int]]:
    g = GraphBuilder(TKR[int], TKR[int])
    graph = g.task(doubler_plus_graph())
    graph_ref = TypedGraphRef(graph.value_ref(), TKR[int], TKR[int])
    out = g.eval(graph_ref, g.inputs)
    return g.finish_with_outputs(out)


class ApplyTwiceInput(NamedTuple):
    # Note we mangle this like the stub generator would, although the generator
    # never sees the graph's inputs as they are hidden in an untyped GraphData.
    graph: TKR[OpaqueType["tierkreis.controller.data.graph.GraphData"]]  # noqa: F821
    value: TKR[int]


def eval_from_worker_with_graph_from_worker() -> FinishedGraph[TKR[int], TKR[int]]:
    g = GraphBuilder(TKR[int], TKR[int])
    graph = g.task(doubler_plus_graph())
    # This is ok, but we can't pass the graph_ref into ApplyTwiceInput
    # graph_ref = TypedGraphRef(graph.value_ref(), TKR[int], TKR[int])
    inputs = ApplyTwiceInput(graph=graph, value=g.inputs)

    ap2 = g.task(apply_twice())
    ap2_ref = TypedGraphRef(ap2.value_ref(), ApplyTwiceInput, TKR[int])
    out = g.eval(ap2_ref, inputs)
    return g.finish_with_outputs(out)


def eval_graph_of_graph() -> FinishedGraph[TKR[int], TKR[int]]:
    g = GraphBuilder(TKR[int], TKR[int])
    graph = g.task(doubler_plus_graph())
    # This is ok, but we can't pass the graph_ref into exponentiate_graph:
    # graph_ref = TypedGraphRef(graph.value_ref(), TKR[int], TKR[int])
    eg = g.task(graph_of_graph(graph, g.const(3)))
    exp_graph = TypedGraphRef(eg.value_ref(), TKR[int], TKR[int])
    out = g.eval(exp_graph, g.inputs)
    return g.finish_with_outputs(out)


def embed_graph():
    class InnerOutput(NamedTuple):
        log: TKR[str]
        nxt: TKR[int]

    class OuterOutput(NamedTuple):
        s1: TKR[str]
        s2: TKR[str]
        final: TKR[int]

    def inner():
        g = GraphBuilder(TKR[int], InnerOutput)
        s = g.task(tkr_str(g.inputs))
        div2 = g.task(idivide(a=g.inputs, b=g.const(2)))
        times3plus1 = g.task(
            iadd(a=g.task(itimes(a=g.inputs, b=g.const(3))), b=g.const(1))
        )
        even = g.task(eq(g.task(mod(a=g.inputs, b=g.const(2))), g.const(0)))
        n = g.ifelse(even, div2, times3plus1)
        return g.finish_with_outputs(InnerOutput(log=s, nxt=n))

    g = GraphBuilder(TKR[int], OuterOutput)
    inner_g = inner()
    first = g.embed(inner_g, g.inputs, InnerOutput)
    second = g.embed(inner_g, first.nxt, InnerOutput)
    return g.finish_with_outputs(OuterOutput(first.log, second.log, second.nxt))
