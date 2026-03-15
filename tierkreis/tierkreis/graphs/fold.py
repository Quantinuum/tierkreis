"""Preconstructed graph for folding operations."""

from typing import NamedTuple, TypeVar

from tierkreis.builder import Graph, Workflow, TypedGraphRef
from tierkreis.builtins import head, igt, tkr_len
from tierkreis.controller.data.models import TKR
from tierkreis.controller.data.types import PType


class _FoldGraphOuterInputs[A: PType, B: PType](NamedTuple):
    func: TKR[Workflow["FoldFunctionInput[A, B]", TKR[B]]]
    accum: TKR[B]
    values: TKR[list[A]]


class _FoldGraphOuterOutputs[A: PType, B: PType](NamedTuple):
    accum: TKR[B]
    values: TKR[list[A]]
    should_continue: TKR[bool]


def _fold_graph_outer[A: PType, B: PType]() -> Workflow[
    _FoldGraphOuterInputs[A, B],
    _FoldGraphOuterOutputs[A, B],
]:
    g = Graph(_FoldGraphOuterInputs[A, B], _FoldGraphOuterOutputs[A, B])

    func = g.inputs.func
    accum = g.inputs.accum
    values = g.inputs.values

    values_len = g.task(tkr_len(values))
    # True if there is more than one value in the list.
    non_empty = g.task(igt(values_len, g.const(0)))

    # Will only succeed if values is non-empty.
    headed = g.task(head(values))

    # Apply the function if we were able to pop off a value.
    tgd = TypedGraphRef[FoldFunctionInput[A, B], TKR[B]](func, TKR[B])
    applied_next = g.eval(tgd, FoldFunctionInput(accum, headed.head))
    next_accum = g.ifelse(non_empty, applied_next, accum)
    next_values = g.ifelse(non_empty, headed.rest, values)
    return g.finish_with_outputs(
        _FoldGraphOuterOutputs(next_accum, next_values, non_empty)
    )


A_co = TypeVar("A_co", bound=PType, covariant=True)
B_co = TypeVar("B_co", bound=PType, covariant=True)


class FoldGraphInputs[A: PType, B: PType](NamedTuple):
    """Inputs to a fold graph.

    :fields:
        initial (B): The initial value.
        values (list[A]): The list of values to fold over.
    """

    initial: TKR[B]
    values: TKR[list[A]]


class FoldFunctionInput[A: PType, B: PType](NamedTuple):
    """Input type of a fold function.

    :fields:
        accum (B): The accumulator.
        value (A): The current value.
    """

    accum: TKR[B]
    value: TKR[A]


def fold_graph[A_co: PType, B_co: PType](
    func: Workflow[FoldFunctionInput[A_co, B_co], TKR[B_co]],
) -> Workflow[FoldGraphInputs[A_co, B_co], TKR[B_co]]:
    """Construct a fold graph.

    fold : {func: (b -> a -> b)} -> {initial: b} -> {values: list[a]} -> {value: b}
    fold : { A x B -> B } -> { list[A] x B -> B }

    :param func: The function to fold over.
    :type func: Graph[FoldFunctionInput[A_co, B_co], TKR[B_co]]
    :return: A graph implementing the fold function.
    :rtype: Graph[FoldGraphInputs[A_co, B_co], TKR[B_co]]
    """
    g = Graph(FoldGraphInputs[A_co, B_co], TKR[B_co])
    foldfunc = g._graph_const(func)  # noqa: SLF001
    # TODO @mwpb: include the computation inside the fold
    ins = _FoldGraphOuterInputs(
        foldfunc.graph_ref,
        g.inputs.initial,
        g.inputs.values,
    )
    loop = g.loop(_fold_graph_outer(), ins)
    return g.finish_with_outputs(loop.accum)
