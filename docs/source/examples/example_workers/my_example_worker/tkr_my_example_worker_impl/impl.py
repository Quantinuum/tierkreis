import logging

from guppylang import comptime, guppy
from guppylang.std.builtins import array, result
from guppylang.std.quantum import cx, h, measure_array, qubit
from hugr.package import Package
from pytket._tket.circuit import Circuit, fresh_symbol
from sympy import Symbol

from tierkreis import Worker

logger = logging.getLogger(__name__)

worker = Worker("my_example_worker")


@worker.task()
def substitute(circuit: Circuit, a: float, b: float, c: float) -> Circuit:
    circuit.symbol_substitution({Symbol("a"): a, Symbol("b"): b, Symbol("c"): c})
    return circuit


@worker.task()
def symbolic_circuit() -> Circuit:
    a = fresh_symbol("a")
    b = fresh_symbol("b")
    c = fresh_symbol("c")
    circ = Circuit(3)
    circ.Rz(a, 0)
    circ.Rz(b, 0)
    circ.Rz(c, 0)
    circ.measure_all()
    return circ


@worker.task()
def ghz(size: int) -> Package:
    n = guppy.nat_var("n")

    @guppy
    def build_ghz_state(q: array[qubit, n]) -> None:  # type: ignore
        h(q[0])
        for i in range(n - 1):  # type: ignore
            cx(q[i], q[i + 1])

    @guppy
    def main() -> None:
        q = array(qubit() for _ in range(comptime(size)))  # type: ignore
        build_ghz_state(q)

        result("c", measure_array(q))

    return main.compile()
