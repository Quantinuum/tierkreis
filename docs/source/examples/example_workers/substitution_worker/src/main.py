import logging
from sys import argv


from tierkreis import Worker
from pytket._tket.circuit import Circuit
from sympy import Symbol

logger = logging.getLogger(__name__)

worker = Worker("substitution_worker")


@worker.task()
def substitute(circuit: Circuit, a: float, b: float, c: float) -> Circuit:
    circuit.symbol_substitution({Symbol("a"): a, Symbol("b"): b, Symbol("c"): c})
    return circuit


if __name__ == "__main__":
    worker.app(argv)
