from typing import NamedTuple

from tierkreis.builder import GraphBuilder, FinishedGraph
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


def omit_input() -> FinishedGraph[Inputs, TKR[list[int]]]:
    g = GraphBuilder(Inputs, TKR[list[int]])
    range_1 = g.task(tkr_range(g.inputs.start, g.inputs.stop))
    return g.outputs(range_1)


def passthru() -> FinishedGraph[Inputs, TKR[list[int]]]:
    g = GraphBuilder(Inputs, TKR[list[int]])
    range_1 = g.task(tkr_range(g.inputs.start, g.inputs.stop, g.inputs.step))
    return g.outputs(range_1)


def defaults_omit() -> FinishedGraph[Inputs, OuterOutputs]:
    g = GraphBuilder(Inputs, OuterOutputs)
    range_1 = g.eval(omit_input(), Inputs(g.inputs.start, g.inputs.stop))
    range_2 = g.eval(passthru(), Inputs(g.inputs.start, g.inputs.stop))
    range_3 = g.eval(passthru(), Inputs(g.inputs.start, g.inputs.stop, g.const(2)))
    return g.outputs(OuterOutputs(range_1, range_2, range_3))


def defaults_passthru() -> FinishedGraph[Inputs, OuterOutputs]:
    g = GraphBuilder(Inputs, OuterOutputs)
    range_1 = g.eval(omit_input(), Inputs(g.inputs.start, g.inputs.stop))
    range_2 = g.eval(passthru(), Inputs(g.inputs.start, g.inputs.stop))
    range_3 = g.eval(passthru(), Inputs(g.inputs.start, g.inputs.stop, g.const(2)))
    return g.outputs(OuterOutputs(range_1, range_2, range_3, g.inputs.step))


def defaults_not_none() -> FinishedGraph[Inputs, OuterOutputs]:
    g = GraphBuilder(Inputs, OuterOutputs)
    range_1 = g.eval(omit_input(), Inputs(g.inputs.start, g.inputs.stop))
    range_2 = g.eval(passthru(), Inputs(g.inputs.start, g.inputs.stop))
    range_3 = g.eval(passthru(), Inputs(g.inputs.start, g.inputs.stop, g.const(2)))
    return g.outputs(OuterOutputs(range_1, range_2, range_3, g.inputs.start))
