from sys import argv

from tierkreis import Worker
from tierkreis.builder import GraphBuilder
from tierkreis.builtins.stubs import iadd, itimes
from tierkreis.controller.data.graph import GraphData
from tierkreis.models import TKR

worker = Worker("graph")


@worker.task()
def doubler_plus_graph() -> GraphData:
    g = GraphBuilder(TKR[int], TKR[int])
    double = g.task(itimes(g.inputs, g.const(2)))
    out = g.task(iadd(double, g.const(1)))
    g.outputs(out)
    return g.get_data()


if __name__ == "__main__":
    worker.app(argv)
