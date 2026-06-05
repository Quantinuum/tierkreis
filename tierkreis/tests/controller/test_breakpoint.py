from pathlib import Path
from uuid import UUID

import pytest
from pytket._tket.circuit import Circuit
from pytket_worker import n_qubits

from tierkreis.builder import Graph
from tierkreis.builtins import iadd
from tierkreis.controller import resume_graph, run_graph
from tierkreis.controller.data.location import Loc
from tierkreis.controller.data.models import TKR
from tierkreis.controller.data.types import Workflow
from tierkreis.controller.executor.in_memory_executor import InMemoryExecutor
from tierkreis.controller.storage.filestorage import ControllerFileStorage
from tierkreis.controller.storage.in_memory import ControllerInMemoryStorage
from tierkreis.executor import ShellExecutor
from tierkreis.storage import read_outputs


def breakpoint_graph() -> Workflow[TKR[Circuit], TKR[int]]:
    g = Graph(TKR[Circuit], TKR[int])
    test = g.const(5)
    nq = g.task(n_qubits(g.inputs), has_breakpoint=True)  # type: ignore
    out = g.task(iadd(test, nq), has_breakpoint=True)
    return g.finish_with_outputs(out)


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
        storage,
        executor,
        graph,
        Circuit(2),
        enable_breakpoints=enable_breakpoints,
        polling_interval_seconds=0.1,
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
