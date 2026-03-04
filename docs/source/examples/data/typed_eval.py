from typing import NamedTuple

from tierkreis.builder import GraphBuilder
from tierkreis.builtins import iadd, itimes
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
    return g.finish_with_outputs(out)


def typed_doubler_plus_multi():
    g = GraphBuilder(DoublerInput, DoublerOutput)
    mul = g.task(itimes(a=g.inputs.x, b=g.const(2)))
    out = g.task(iadd(a=mul, b=g.inputs.intercept))
    return g.finish_with_outputs(DoublerOutput(a=g.inputs.x, value=out))


def typed_doubler_plus():
    g = GraphBuilder(DoublerInput, TKR[int])
    mul = g.task(itimes(a=g.inputs.x, b=g.const(2)))
    out = g.task(iadd(a=mul, b=g.inputs.intercept))
    return g.finish_with_outputs(out)


class TypedEvalOutputs(NamedTuple):
    typed_eval_output: TKR[int]


def typed_eval():
    g = GraphBuilder(EmptyModel, TypedEvalOutputs)
    e = g.eval(typed_doubler_plus(), DoublerInput(x=g.const(6), intercept=g.const(0)))
    return g.finish_with_outputs(TypedEvalOutputs(typed_eval_output=e))
