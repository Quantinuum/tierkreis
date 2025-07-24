from pathlib import Path
from typing import Any, Type
from uuid import UUID

import pytest
from tests.controller.loop_graphdata import loop_multiple_acc, loop_multiple_acc_untyped
from tierkreis.controller import run_graph
from tierkreis.controller.executor.in_memory_executor import InMemoryExecutor
from tierkreis.controller.executor.shell_executor import ShellExecutor
from tierkreis.controller.storage.filestorage import ControllerFileStorage
from tierkreis.controller.storage.in_memory import ControllerInMemoryStorage
from tierkreis.storage import read_loop_trace
from tierkreis_core import GraphData


return_value = [
    {"acc1": x, "acc2": y, "acc3": z}
    for x, y, z in zip(range(1, 7), range(2, 13, 2), range(3, 19, 3))
]

params: list[tuple[GraphData, Any, str, int]] = [
    (
        loop_multiple_acc_untyped(),
        return_value,
        "multi_acc",
        9,
    ),
    (
        loop_multiple_acc().get_data(),
        return_value,
        "multi_acc",
        9,
    ),
]
ids = [
    "loop_multiple_acc_untyped",
    "loop_multiple_acc",
]

storage_classes = [ControllerFileStorage, ControllerInMemoryStorage]
storage_ids = ["FileStorage", "In-memory"]


@pytest.mark.parametrize("storage_class", storage_classes, ids=storage_ids)
@pytest.mark.parametrize("graph,output,name,id", params, ids=ids)
def test_read_loop_trace(
    storage_class: Type[ControllerFileStorage | ControllerInMemoryStorage],
    graph: GraphData,
    output: Any,
    name: str,
    id: int,
):
    g = graph
    storage = storage_class(UUID(int=id), name=name)
    executor = ShellExecutor(Path("./python/examples/launchers"), Path(""))
    if isinstance(storage, ControllerInMemoryStorage):
        executor = InMemoryExecutor(Path("./tierkreis/tierkreis"), storage=storage)
    storage.clean_graph_files()
    run_graph(storage, executor, g, {})

    actual_output = read_loop_trace(g, storage, "my_loop")
    assert actual_output == output
