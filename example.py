from uuid import UUID

from tierkreis.builder import GraphBuilder
from tierkreis.builtins import iadd
from tierkreis.controller import run_graph
from tierkreis.controller.data.models import TKR
from tierkreis.executor import ShellExecutor
from tierkreis.storage import FileStorage


def simple_graph():
    g = GraphBuilder(TKR[int], TKR[int])
    test = g.const(5)
    test_after_break = g.breakpoint(test)
    out = g.task(iadd(test_after_break, g.inputs))
    g.outputs(out)
    return g


def main() -> None:
    storage = FileStorage(UUID(int=222), name="test")
    executor = ShellExecutor(registry_path=None, workflow_dir=storage.workflow_dir)
    graph = simple_graph()
    storage.clean_graph_files()
    run_graph(storage, executor, graph, graph_inputs={"value": 1})
    # resume_graph(storage, executor)
    # print(read_outputs(graph, storage))


if __name__ == "__main__":
    main()
