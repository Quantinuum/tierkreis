from typing import NamedTuple

from tierkreis.builder import Graph
from tierkreis.builtins import tkr_range
from tierkreis.models import TKR, Workflow


class Inputs(NamedTuple):
    start: TKR[int]
    stop: TKR[int]
    step: TKR[int] | None = None


class OuterOutputs(NamedTuple):
    range_1: TKR[list[int]]
    range_2: TKR[list[int]]
    range_3: TKR[list[int]]
    range_4: TKR[list[int]]


def omit_input() -> Workflow[Inputs, TKR[list[int]]]:
    g = Graph(Inputs, TKR[list[int]])
    range_1 = g.task(tkr_range(g.inputs.start, g.inputs.stop))
    return g.finish_with_outputs(range_1)


def passthru() -> Workflow[Inputs, TKR[list[int]]]:
    g = Graph(Inputs, TKR[list[int]])
    range_1 = g.task(tkr_range(g.inputs.start, g.inputs.stop, g.inputs.step))
    return g.finish_with_outputs(range_1)


def defaults_omit() -> Workflow[Inputs, OuterOutputs]:
    g = Graph(Inputs, OuterOutputs)
    range_1 = g.task(tkr_range(g.inputs.start, g.inputs.stop))
    range_2 = g.task(tkr_range(g.inputs.start, g.inputs.stop, g.const(2)))
    range_3 = g.eval(omit_input(), Inputs(g.inputs.start, g.inputs.stop))
    range_4 = g.eval(omit_input(), Inputs(g.inputs.start, g.inputs.stop, g.const(2)))
    return g.finish_with_outputs(OuterOutputs(range_1, range_2, range_3, range_4))


def defaults_passthru() -> Workflow[Inputs, OuterOutputs]:
    g = Graph(Inputs, OuterOutputs)
    range_1 = g.task(tkr_range(g.inputs.start, g.inputs.stop))
    range_2 = g.task(tkr_range(g.inputs.start, g.inputs.stop, g.const(2)))
    # Evaluating passthru results in a default value of None being set for step
    # which results in a type error. The graph builder does not currently
    # support setting integer value defaults that would avoid this.
    # range_3 = g.eval(passthru(), Inputs(g.inputs.start, g.inputs.stop))
    range_3 = g.const([])
    range_4 = g.eval(passthru(), Inputs(g.inputs.start, g.inputs.stop, g.const(2)))
    return g.finish_with_outputs(OuterOutputs(range_1, range_2, range_3, range_4))
