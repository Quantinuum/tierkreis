from typing import NamedTuple

from tests.workers.graph.api.api import doubler_plus_graph
from tierkreis.builder import GraphBuilder, TypedGraphRef
from tierkreis.builtins.stubs import (
    conjugate,
    iadd,
    igt,
    itimes,
    mod,
    tkr_tuple,
    untuple,
)
from tierkreis.controller.data.core import EmptyModel
from tierkreis.controller.data.models import TKR


class DoublerInput(NamedTuple):
    x: TKR[int]
    intercept: TKR[int]


class DoublerOutput(NamedTuple):
    a: TKR[int]
    value: TKR[int]


def typed_doubler():
    g = GraphBuilder(TKR[int], TKR[int])
    out = g.task(itimes(a=g.const(2), b=g.inputs))
    g.outputs(out)
    return g


def typed_doubler_plus_multi():
    g = GraphBuilder(DoublerInput, DoublerOutput)
    mul = g.task(itimes(a=g.inputs.x, b=g.const(2)))
    out = g.task(iadd(a=mul, b=g.inputs.intercept))
    g.outputs(DoublerOutput(a=g.inputs.x, value=out))
    return g


def typed_doubler_plus():
    g = GraphBuilder(DoublerInput, TKR[int])
    mul = g.task(itimes(a=g.inputs.x, b=g.const(2)))
    out = g.task(iadd(a=mul, b=g.inputs.intercept))
    g.outputs(out)
    return g


class TypedEvalOutputs(NamedTuple):
    typed_eval_output: TKR[int]


def typed_eval():
    g = GraphBuilder(EmptyModel, TypedEvalOutputs)
    e = g.eval(typed_doubler_plus(), DoublerInput(x=g.const(6), intercept=g.const(0)))
    g.outputs(TypedEvalOutputs(typed_eval_output=e))
    return g


class LoopBodyInput(NamedTuple):
    loop_acc: TKR[int]


class LoopBodyOutput(NamedTuple):
    loop_acc: TKR[int]
    should_continue: TKR[bool]


def loop_body():
    g = GraphBuilder(LoopBodyInput, LoopBodyOutput)
    a_plus = g.task(iadd(a=g.inputs.loop_acc, b=g.const(1)))
    pred = g.task(igt(a=g.const(10), b=a_plus))
    g.outputs(LoopBodyOutput(loop_acc=a_plus, should_continue=pred))
    return g


def typed_loop():
    g = GraphBuilder(EmptyModel, TKR[int])
    loop = g.loop(loop_body(), LoopBodyInput(loop_acc=g.const(6)))
    g.outputs(loop.loop_acc)
    return g


def typed_map_simple():
    g = GraphBuilder(TKR[list[int]], TKR[list[int]])
    m = g.map(typed_doubler(), g.inputs)
    g.outputs(m)
    return g


def typed_map():
    g = GraphBuilder(TKR[list[int]], TKR[list[int]])
    ins = g.map(lambda n: DoublerInput(x=n, intercept=g.const(6)), g.inputs)
    m = g.map(typed_doubler_plus(), ins)
    g.outputs(m)
    return g


def typed_destructuring():
    g = GraphBuilder(TKR[list[int]], TKR[list[int]])
    ins = g.map(lambda n: DoublerInput(x=n, intercept=g.const(6)), g.inputs)
    m = g.map(typed_doubler_plus_multi(), ins)
    mout = g.map(lambda x: x.value, m)
    g.outputs(mout)
    return g


def tuple_untuple():
    g = GraphBuilder(EmptyModel, TKR[int])
    t = g.task(tkr_tuple(g.const(1), g.const(2)))
    ut = g.task(untuple(t))
    g.outputs(g.task(iadd(ut.a, ut.b)))
    return g


def factorial():
    g = GraphBuilder(TKR[int], TKR[int])
    pred = g.task(igt(g.inputs, g.const(1)))
    n_minus_one = g.task(iadd(g.const(-1), g.inputs))
    rec = g.eval(g.ref(), n_minus_one)
    out = g.ifelse(pred, g.task(itimes(g.inputs, rec)), g.const(1))
    g.outputs(out)
    return g


class GCDInput(NamedTuple):
    a: TKR[int]
    b: TKR[int]


def gcd():
    g = GraphBuilder(GCDInput, TKR[int])

    pred = g.task(igt(g.inputs.b, g.const(0)))
    a_mod_b = g.task(mod(g.inputs.a, g.inputs.b))
    rec = g.eval(g.ref(), GCDInput(a=g.inputs.b, b=a_mod_b))

    g.outputs(g.ifelse(pred, rec, g.inputs.a))
    return g


def tkr_conj():
    g = GraphBuilder(TKR[complex], TKR[complex])
    z = g.task(conjugate(g.inputs))
    g.outputs(z)
    return g


def tkr_list_conj():
    g = GraphBuilder(TKR[list[complex]], TKR[list[complex]])
    zs = g.map(tkr_conj(), g.inputs)
    g.outputs(zs)
    return g


def eval_body_is_from_worker():
    g = GraphBuilder(TKR[int], TKR[int])
    graph = g.task(doubler_plus_graph())
    graph_ref = TypedGraphRef(graph.value_ref(), TKR[int], TKR[int])
    out = g.eval(graph_ref, g.inputs)
    g.outputs(out)
    return g
