from typing import NamedTuple

from tierkreis.builder import GraphBuilder
from tierkreis.builtins import tkr_range
from tierkreis.controller.data.models import TKR


class Inputs(NamedTuple):
    start: TKR[int]
    stop: TKR[int]
    step: TKR[int] | None = None


class OuterOutputs(NamedTuple):
    range_1: TKR[list[int]]
    range_2: TKR[list[int]]
    range_3: TKR[list[int]]
    extra_output: TKR[int] | None = None


def omit_input() -> GraphBuilder[Inputs, TKR[list[int]]]:
    g = GraphBuilder(Inputs, TKR[list[int]])
    range_1 = g.task(tkr_range(g.inputs.start, g.inputs.stop))
    g.outputs(range_1)
    return g


def passthru() -> GraphBuilder[Inputs, TKR[list[int]]]:
    g = GraphBuilder(Inputs, TKR[list[int]])
    range_1 = g.task(tkr_range(g.inputs.start, g.inputs.stop, g.inputs.step))
    g.outputs(range_1)
    return g


def defaults_omit() -> GraphBuilder[Inputs, OuterOutputs]:
    g = GraphBuilder(Inputs, OuterOutputs)
    range_1 = g.eval(omit_input(), Inputs(g.inputs.start, g.inputs.stop))
    range_2 = g.eval(passthru(), Inputs(g.inputs.start, g.inputs.stop))
    range_3 = g.eval(passthru(), Inputs(g.inputs.start, g.inputs.stop, g.const(2)))
    g.outputs(OuterOutputs(range_1, range_2, range_3))
    return g


def defaults_passthru() -> GraphBuilder[Inputs, OuterOutputs]:
    g = GraphBuilder(Inputs, OuterOutputs)
    range_1 = g.eval(omit_input(), Inputs(g.inputs.start, g.inputs.stop))
    range_2 = g.eval(passthru(), Inputs(g.inputs.start, g.inputs.stop))
    range_3 = g.eval(passthru(), Inputs(g.inputs.start, g.inputs.stop, g.const(2)))
    g.outputs(OuterOutputs(range_1, range_2, range_3, g.inputs.step))
    return g


def defaults_not_none() -> GraphBuilder[Inputs, OuterOutputs]:
    g = GraphBuilder(Inputs, OuterOutputs)
    range_1 = g.eval(omit_input(), Inputs(g.inputs.start, g.inputs.stop))
    range_2 = g.eval(passthru(), Inputs(g.inputs.start, g.inputs.stop))
    range_3 = g.eval(passthru(), Inputs(g.inputs.start, g.inputs.stop, g.const(2)))
    g.outputs(OuterOutputs(range_1, range_2, range_3, g.inputs.start))
    return g
