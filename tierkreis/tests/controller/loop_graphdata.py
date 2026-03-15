from typing import NamedTuple

import tierkreis.builtins.stubs as tkr_builtins
from tierkreis.builder import GraphBuilder, Workflow
from tierkreis.controller.data.core import EmptyModel
from tierkreis.controller.data.graph import GraphData
from tierkreis.models import TKR


def _loop_body_multiple_acc_untyped() -> GraphData:
    g = GraphData()

    acc = g.input("acc1")
    acc2 = g.input("acc2")
    acc3 = g.input("acc3")

    one = g.const(1)
    two = g.const(2)
    three = g.const(3)
    five = g.const(5)

    should_continue = g.func("builtins.igt", {"a": five, "b": acc})("value")

    new_acc = g.func("builtins.iadd", {"a": acc, "b": one})("value")
    new_acc2 = g.func("builtins.iadd", {"a": acc2, "b": two})("value")
    new_acc3 = g.func("builtins.iadd", {"a": acc3, "b": three})("value")

    g.output(
        {
            "should_continue": should_continue,
            "acc1": new_acc,
            "acc2": new_acc2,
            "acc3": new_acc3,
        },
    )

    return g


def loop_multiple_acc_untyped() -> GraphData:
    g = GraphData()

    acc1 = g.const(0)
    acc2 = g.const(0)
    acc3 = g.const(0)

    body_const = g.const(_loop_body_multiple_acc_untyped())

    loop = g.loop(
        body_const,
        {"acc1": acc1, "acc2": acc2, "acc3": acc3},
        "should_continue",
        "my_loop",
    )

    g.output({"acc1": loop("acc1"), "acc2": loop("acc2"), "acc3": loop("acc3")})

    return g


class MultipleAcc(NamedTuple):
    acc1: TKR[int]
    acc2: TKR[int]
    acc3: TKR[int]


class MultipleAccOut(NamedTuple):
    should_continue: TKR[bool]
    acc1: TKR[int]
    acc2: TKR[int]
    acc3: TKR[int]


def _loop_body_multiple_acc() -> Workflow[MultipleAcc, MultipleAccOut]:
    g = GraphBuilder(MultipleAcc, MultipleAccOut)

    acc = g.inputs.acc1
    acc2 = g.inputs.acc2
    acc3 = g.inputs.acc3

    one = g.const(1)
    two = g.const(2)
    three = g.const(3)
    five = g.const(5)

    should_continue = g.task(tkr_builtins.igt(a=five, b=acc))

    new_acc = g.task(tkr_builtins.iadd(a=acc, b=one))
    new_acc2 = g.task(tkr_builtins.iadd(a=acc2, b=two))
    new_acc3 = g.task(tkr_builtins.iadd(a=acc3, b=three))

    return g.finish_with_outputs(
        MultipleAccOut(
            should_continue=should_continue,
            acc1=new_acc,
            acc2=new_acc2,
            acc3=new_acc3,
        ),
    )


class LoopMultipleAccOut(NamedTuple):
    acc1: TKR[int]
    acc2: TKR[int]
    acc3: TKR[int]


def loop_multiple_acc() -> Workflow[EmptyModel, LoopMultipleAccOut]:
    g = GraphBuilder(EmptyModel, LoopMultipleAccOut)

    acc1 = g.const(0)
    acc2 = g.const(0)
    acc3 = g.const(0)

    body = _loop_body_multiple_acc()
    loop = g.loop(body, MultipleAcc(acc1, acc2, acc3), "my_loop")

    return g.finish_with_outputs(
        LoopMultipleAccOut(acc1=loop.acc1, acc2=loop.acc2, acc3=loop.acc3)
    )


class Scoping(NamedTuple):
    current: TKR[int]
    end: TKR[int]


class ScopingOut(NamedTuple):
    should_continue: TKR[bool]
    current: TKR[int]


def _loop_body_scoping() -> Workflow[Scoping, ScopingOut]:
    g = GraphBuilder(Scoping, ScopingOut)

    one = g.const(1)

    next_val = g.task(tkr_builtins.iadd(g.inputs.current, one))
    should_continue = g.task(tkr_builtins.neq(g.inputs.end, g.inputs.current))

    return g.finish_with_outputs(
        ScopingOut(should_continue=should_continue, current=next_val)
    )


class LoopScopingOut(NamedTuple):
    result: TKR[int]


def loop_scoping() -> Workflow[EmptyModel, LoopScopingOut]:
    g = GraphBuilder(EmptyModel, LoopScopingOut)

    start = g.const(0)
    end = g.const(10)

    body = _loop_body_scoping()
    loop = g.loop(body, Scoping(start, end), "scoped_loop")

    return g.finish_with_outputs(LoopScopingOut(result=loop.current))
