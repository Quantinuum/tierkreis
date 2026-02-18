from pathlib import Path
from uuid import UUID

from tierkreis import run_graph
from tierkreis.builder import GraphBuilder
from tierkreis.builtins.stubs import iadd, itimes
from tierkreis.controller.data.location import Loc
from tierkreis.controller.data.models import TKR
from tierkreis.executor import UvExecutor
from tierkreis.storage import FileStorage, read_outputs


def test_restart():
    storage = FileStorage(UUID(int=300), "test_restart")
    storage.clean_graph_files()
    executor = UvExecutor(Path(__file__).parent.parent / "workers", storage.logs_path)

    g = GraphBuilder(TKR[int], TKR[int])
    plus_one = g.task(iadd(g.const(1), g.inputs))
    left = g.task(iadd(g.const(1), plus_one))
    right = g.task(itimes(g.const(2), plus_one))
    out = g.task(iadd(left, right))
    g.outputs(out)

    run_graph(storage, executor, g, {"value": 0})

    outputs = read_outputs(g, storage)
    assert outputs == 4

    invalidated = storage.restart_task(Loc().N(plus_one.node_index))
    expected = [
        Loc().N(left.node_index),
        Loc().N(right.node_index),
        Loc().N(out.node_index),
        Loc().N(g.get_data().output_idx()),
    ]
    assert sorted(invalidated) == sorted(expected)
