from tierkreis import Labels
from tierkreis_core import GraphData


def doubler_plus() -> GraphData:
    g = GraphData()
    inp = g.input("doubler_input")
    intercept = g.input("intercept")
    two = g.const(2)
    mul = g.func("builtins.itimes", {"a": inp, "b": two})("value")
    out = g.func("builtins.iadd", {"a": mul, "b": intercept})("value")
    g.output({"doubler_output": out})
    return g


def simple_eval() -> GraphData:
    g = GraphData()
    zero = g.const(0)
    six = g.const(6)
    doubler_const = g.const(doubler_plus())
    e = g.eval(doubler_const, {"doubler_input": six, "intercept": zero})
    g.output({"simple_eval_output": e("doubler_output")})
    return g


def simple_partial() -> GraphData:
    g = GraphData()
    zero = g.const(0)
    six = g.const(6)
    doubler_const = g.const(doubler_plus())
    partial = g.eval(doubler_const, {"intercept": zero})("body")
    e = g.eval(partial, {"doubler_input": six})
    g.output({"value": e("doubler_output")})
    return g


def loop_body() -> GraphData:
    g = GraphData()
    a = g.input("loop_acc")
    one = g.const(1)
    N = g.const(10)

    a_plus = g.func("builtins.iadd", {"a": a, "b": one})("value")
    pred = g.func("builtins.igt", {"a": N, "b": a_plus})("value")
    g.output({"loop_acc": a_plus, "should_continue": pred})
    return g


def simple_loop() -> GraphData:
    g = GraphData()
    six = g.const(6)
    g_const = g.const(loop_body())
    loop = g.loop(g_const, {"loop_acc": six}, "should_continue")
    g.output({"value": loop("loop_acc")})
    return g


def simple_map() -> GraphData:
    g = GraphData()
    six = g.const(6)
    Ns_const = g.const(list(range(21)))
    Ns = g.func("builtins.unfold_values", {Labels.VALUE: Ns_const})
    doubler_const = g.const(doubler_plus())

    m = g.map(doubler_const, {"doubler_input": Ns("*"), "intercept": six})
    folded = g.func("builtins.fold_values", {"values_glob": m("*")})
    g.output({"value": folded(Labels.VALUE)})
    return g


def simple_ifelse() -> GraphData:
    g = GraphData()
    pred = g.input("pred")
    one = g.const(1)
    two = g.const(2)
    out = g.if_else(pred, one, two)("value")
    g.output({"value": out})
    return g


def maps_in_series() -> GraphData:
    g = GraphData()
    zero = g.const(0)

    Ns_const = g.const(list(range(21)))
    Ns = g.func("builtins.unfold_values", {Labels.VALUE: Ns_const})
    doubler_const = g.const(doubler_plus())

    m = g.map(doubler_const, {"doubler_input": Ns("*"), "intercept": zero})

    m2 = g.map(doubler_const, {"doubler_input": m("*"), "intercept": zero})
    folded = g.func("builtins.fold_values", {"values_glob": m2("*")})
    g.output({"value": folded(Labels.VALUE)})
    return g


def map_with_str_keys() -> GraphData:
    g = GraphData()
    zero = g.const(0)
    Ns_const = g.const({"one": 1, "two": 2, "three": 3})
    Ns = g.func("builtins.unfold_dict", {Labels.VALUE: Ns_const})
    doubler_const = g.const(doubler_plus())

    m = g.map(doubler_const, {"doubler_input": Ns("*"), "intercept": zero})
    folded = g.func("builtins.fold_dict", {"values_glob": m("*")})
    g.output({"value": folded(Labels.VALUE)})
    return g


def factorial() -> GraphData:
    g = GraphData()
    minus_one = g.const(-1)
    one = g.const(1)
    n = g.input("n")
    fac = g.input("factorial")

    pred = g.func("builtins.igt", {"a": n, "b": one})("value")

    n_minus_one = g.func("builtins.iadd", {"a": minus_one, "b": n})("value")
    rec = g.eval(fac, {"n": n_minus_one, "factorial": fac})("factorial_output")
    if_true = g.func("builtins.itimes", {"a": n, "b": rec})("value")

    if_false = g.const(1)

    out = g.if_else(pred, if_true, if_false)("value")
    g.output({"factorial_output": out})
    return g


def simple_eagerifelse() -> GraphData:
    g = GraphData()
    pred = g.input("pred")
    one = g.const(1)
    two = g.const(2)
    out = g.eager_if_else(pred, one, two)(Labels.VALUE)
    g.output({"value": out})
    return g
