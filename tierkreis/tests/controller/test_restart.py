from pathlib import Path
from uuid import UUID

import pytest
from tierkreis._tierkreis import new_default

from tierkreis import run_graph
from tierkreis.builder import Graph
from tierkreis.builtins import iadd, itimes
from tierkreis.controller.data.location import Loc
from tierkreis.controller.data.models import TKR
from tierkreis.executor import UvExecutor
from tierkreis.storage import FileStorage, read_outputs


def test_restart() -> None:
    storage = FileStorage(UUID(int=300), "test_restart")
    storage.clean_graph_files()
    executor = UvExecutor(Path(__file__).parent.parent / "workers", storage.logs_path)

    g = Graph(TKR[int], TKR[int])
    plus_one = g.task(iadd(g.const(1), g.inputs))
    left = g.task(iadd(g.const(1), plus_one))
    right = g.task(itimes(g.const(2), plus_one))
    out = g.task(iadd(left, right))
    g = g.finish_with_outputs(out)

    run_graph(storage, executor, g, {"value": 0})

    outputs = read_outputs(g, storage)
    assert outputs == 4

    invalidated = storage.restart_task(Loc().N(plus_one.node_index))
    expected = [
        Loc().N(left.node_index),
        Loc().N(right.node_index),
        Loc().N(out.node_index),
        Loc().N(g.data.output_idx()),
    ]
    assert sorted(invalidated) == sorted(expected)

    run_graph(storage, executor, g, {"value": 0})
    outputs = read_outputs(g, storage)
    assert outputs == 4


@pytest.mark.asyncio
async def test_restart_runtime() -> None:

    g = Graph(TKR[int], TKR[int])
    plus_one = g.task(iadd(g.const(1), g.inputs))
    left = g.task(iadd(g.const(1), plus_one))
    right = g.task(itimes(g.const(2), plus_one))
    out = g.task(iadd(left, right))
    g = g.finish_with_outputs(out)
    runtime = await new_default()
    with runtime:
        workflow_id = await runtime.save_workflow("restart_diamond", g)
        run_id = await runtime.start_new_run(workflow_id, {"value": 0})
        await runtime.wait_for(run_id, timeout=30)
        assert await runtime.get_outputs(run_id) == 4

        new_attempt, invalidated = await runtime.restart_task(
            run_id, [f"N{plus_one.node_index}"]
        )
        assert new_attempt != 0
        # In the new implementation roos it not represent by -.

        expected = {
            f"N{plus_one.node_index}",
            f"N{left.node_index}",
            f"N{right.node_index}",
            f"N{out.node_index}",
            f"N{g.data.output_idx()}",  # We also explicitly invalidate the output of the graph
        }
        assert sorted(invalidated) == sorted(expected)

        await runtime.wait_for(run_id, new_attempt, timeout=30)
        assert await runtime.get_outputs(run_id, new_attempt) == 4

        # The source attempt must remain untouched.
        assert await runtime.get_outputs(run_id) == 4
