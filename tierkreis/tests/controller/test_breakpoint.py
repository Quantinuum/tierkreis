from pathlib import Path
from uuid import UUID

import pytest
from pytket._tket.circuit import Circuit
from tierkreis.builder import GraphBuilder
from tierkreis.builtins import iadd
from tierkreis.controller import resume_graph, run_graph
from tierkreis.controller.data.graph import NodeMetaData
from tierkreis.controller.data.location import Loc
from tierkreis.controller.data.models import TKR
from tierkreis.controller.executor.in_memory_executor import InMemoryExecutor
from tierkreis.controller.storage.filestorage import ControllerFileStorage
from tierkreis.controller.storage.in_memory import ControllerInMemoryStorage
from tierkreis.executor import ShellExecutor
from pytket_worker import n_qubits
from tierkreis.storage import read_outputs


def breakpoint_graph() -> GraphBuilder[TKR[Circuit], TKR[int]]:
    g = GraphBuilder(TKR[Circuit], TKR[int])
    test = g.const(5)
    nq = g.task(n_qubits(g.inputs), NodeMetaData(has_breakpoint=True))  # type: ignore
    out = g.task(iadd(test, nq), NodeMetaData(has_breakpoint=True))
    g.outputs(out)
    return g


storage_classes = [ControllerFileStorage, ControllerInMemoryStorage]
storage_ids = ["FileStorage", "In-memory"]


@pytest.mark.parametrize("storage_class", storage_classes, ids=storage_ids)
@pytest.mark.parametrize("enable_breakpoints", [True, False], ids=["True", "False"])
def test_breakpoint(
    storage_class: type[ControllerFileStorage | ControllerInMemoryStorage],
    enable_breakpoints: bool,
) -> None:
    graph = breakpoint_graph()
    storage = storage_class(UUID(int=400), name="breakpoints")
    executor = ShellExecutor(registry_path=None, workflow_dir=storage.workflow_dir)
    if isinstance(storage, ControllerInMemoryStorage):
        executor = InMemoryExecutor(Path("./tierkreis/tierkreis"), storage=storage)
    storage.clean_graph_files()
    run_graph(
        storage, executor, graph, Circuit(2), enable_breakpoints=enable_breakpoints
    )
    if enable_breakpoints:
        assert not storage.is_node_finished(Loc())
        assert storage.exists(storage._breakpoint(Loc("-.N2")))
        resume_graph(storage, executor, enable_breakpoints=enable_breakpoints)
        assert not storage.is_node_finished(Loc())
        assert storage.exists(storage._breakpoint(Loc("-.N3")))
        resume_graph(storage, executor, enable_breakpoints=enable_breakpoints)
    assert storage.is_node_finished(Loc())
    out = read_outputs(graph, storage)
    assert out == 7
