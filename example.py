from pathlib import Path
from uuid import UUID
from tierkreis.builder import GraphBuilder
from tierkreis.controller import run_graph
from tierkreis.controller.data.models import TKR
from tierkreis.controller.executor.uv_executor import UvExecutor
from tierkreis.storage import FileStorage
from tierkreis.storage import read_outputs


from tierkreis.builtins.stubs import tkr_str


def simple_graph() -> GraphBuilder[TKR[int], TKR[str]]:
    g = GraphBuilder(TKR[int], TKR[str])
    a = g.task(tkr_str(g.inputs))
    g.outputs(a)
    return g


def main() -> None:
    graph_a = simple_graph().data
    graph_b = simple_graph().data
    graph = graph_a + graph_b

    inputs = {"value": 0}
    storage = FileStorage(
        UUID(int=222),
        name="serial_graph",
    )
    executor = UvExecutor(
        Path(__file__).parent / "tierkreis_workers", storage.logs_path
    )
    storage.clean_graph_files()
    run_graph(
        storage,
        executor,
        graph,
        inputs,
    )
    res = read_outputs(graph, storage)
    print(res)
    graph = graph_a @ graph_b
    storage = FileStorage(
        UUID(int=223),
        name="parallel_graph",
    )
    executor = UvExecutor(
        Path(__file__).parent / "tierkreis_workers", storage.logs_path
    )
    storage.clean_graph_files()
    run_graph(
        storage,
        executor,
        graph,
        inputs,
    )
    res = read_outputs(graph, storage)
    print(res)


if __name__ == "__main__":
    main()
    print("All Done")
