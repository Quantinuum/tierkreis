import logging

from pytket import Circuit
from sympy import Symbol

from tierkreis import Worker

logger = logging.getLogger(__name__)

worker = Worker("substitution_worker")


@worker.task()
def substitute(circuit: Circuit, a: float, b: float, c: float) -> Circuit:
    circuit.symbol_substitution({Symbol("a"): a, Symbol("b"): b, Symbol("c"): c})
    return circuit
