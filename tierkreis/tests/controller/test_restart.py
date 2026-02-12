from pathlib import Path
from uuid import UUID

import pytest

from tierkreis import run_graph
from tierkreis.builder import GraphBuilder
from tierkreis.builtins.stubs import iadd
from tierkreis.controller.data.location import Loc
from tierkreis.controller.data.models import TKR
from tierkreis.executor import UvExecutor
from tierkreis.storage import FileStorage, read_outputs


def diamond_graph() -> GraphBuilder[TKR[int], TKR[int]]:
    g = GraphBuilder(TKR[int], TKR[int])
    plus_one = g.task(iadd(g.const(1), g.inputs))
    left = g.task(iadd(g.const(1), plus_one))
    right = g.task(iadd(g.const(1), plus_one))
    out = g.task(iadd(left, right))
    g.outputs(out)
    return g


def test_restart():
    storage = FileStorage(UUID(int=300), "test_restart")
    storage.clean_graph_files()
    executor = UvExecutor(Path(__file__).parent.parent / "workers", storage.logs_path)

    run_graph(storage, executor, diamond_graph(), {"value": 0})

    outputs = read_outputs(diamond_graph(), storage)
    assert outputs == 4

    invalidated = storage.restart_task(Loc().N(2))
    expected = [Loc().N(4), Loc().N(6), Loc().N(7), Loc().N(8)]
    assert sorted(invalidated) == sorted(expected)
