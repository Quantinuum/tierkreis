import logging

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
