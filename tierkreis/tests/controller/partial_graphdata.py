from tierkreis_core import GraphData


def ternary_add() -> GraphData:
    g = GraphData()
    a = g.input("a")
    b = g.input("b")
    c = g.input("c")

    step_one = g.func("builtins.iadd", {"a": a, "b": b})("value")
    step_two = g.func("builtins.iadd", {"a": step_one, "b": c})("value")

    g.output({"ternary_add_output": step_two})
    return g


def double_partial() -> GraphData:
    g = GraphData()

    one = g.const(1)
    two = g.const(2)
    three = g.const(3)
    termary_add_const = g.const(ternary_add())

    add_add_1 = g.eval(termary_add_const, {"a": one})("body")
    add_3 = g.eval(add_add_1, {"b": two})("body")
    six = g.eval(add_3, {"c": three})("ternary_add_output")

    g.output({"double_partial_output": six})
    return g


def partial_intersection() -> GraphData:
    g = GraphData()
    one = g.const(1)
    two = g.const(2)
    three = g.const(3)
    termary_add_const = g.const(ternary_add())

    add_add_1 = g.eval(termary_add_const, {"a": one})("body")
    add_3 = g.eval(add_add_1, {"a": one, "b": two})("body")
    six = g.eval(add_3, {"c": three})("ternary_add_output")

    g.output({"double_partial_output": six})
    return g
