from pathlib import Path
from uuid import UUID

import pytest
from tierkreis._tierkreis import new_default

from tests.controller.typed_graphdata import (
    DoublerInput,
    LoopBodyInput,
    loop_body,
    typed_doubler_plus,
)
from tierkreis import run_graph
from tierkreis.builder import Graph
from tierkreis.builtins import iadd, itimes
from tierkreis.controller.data.location import Loc
from tierkreis.controller.data.models import TKR
from tierkreis.executor import UvExecutor
from tierkreis.models import EmptyModel
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


@pytest.mark.asyncio
async def test_restart_runtime_loop() -> None:
    body = loop_body()
    body_task_index = next(
        index
        for index, node in enumerate(body.data.nodes)
        if getattr(node, "function_name", None) == "builtins.iadd"
    )
    g = Graph(EmptyModel, TKR[int])
    loop = g.loop(body, LoopBodyInput(loop_acc=g.const(6)))
    g = g.finish_with_outputs(loop.loop_acc)
    loop_location = f"N{loop.loop_acc.node_index}.L1.N{body_task_index}"

    runtime = await new_default()
    with runtime:
        workflow_id = await runtime.save_workflow("restart_loop", g)
        run_id = await runtime.start_new_run(workflow_id, {})
        await runtime.wait_for(run_id, timeout=30)
        assert await runtime.get_outputs(run_id) == 10

        new_attempt, invalidated = await runtime.restart_task(run_id, [loop_location])
        assert new_attempt != 0
        assert loop_location in invalidated
        assert f"N{g.data.output_idx()}" in invalidated
        assert f"N{loop.loop_acc.node_index}.L0.N{body_task_index}" not in invalidated

        await runtime.wait_for(run_id, new_attempt, timeout=30)
        assert await runtime.get_outputs(run_id, new_attempt) == 10
        assert await runtime.get_outputs(run_id) == 10


@pytest.mark.asyncio
async def test_restart_runtime_map() -> None:
    body = typed_doubler_plus()
    body_task_index = next(
        index
        for index, node in enumerate(body.data.nodes)
        if getattr(node, "function_name", None) == "builtins.iadd"
    )
    g = Graph(TKR[list[int]], TKR[list[int]])
    six = g.const(6)
    mapped_inputs = g.map(
        lambda value: DoublerInput(x=value, intercept=six),
        g.inputs,
    )
    mapped = g.map(body, mapped_inputs)
    map_node_index = next(
        index
        for index, node in enumerate(g.data.nodes)
        if getattr(node, "type", None) == "map"
    )
    g = g.finish_with_outputs(mapped)
    map_location = f"N{map_node_index}.M0.N{body_task_index}"

    runtime = await new_default()
    with runtime:
        workflow_id = await runtime.save_workflow("restart_map", g)
        run_id = await runtime.start_new_run(workflow_id, {"value": [0, 1, 2]})
        await runtime.wait_for(run_id, timeout=30)
        assert await runtime.get_outputs(run_id) == [6, 8, 10]

        new_attempt, invalidated = await runtime.restart_task(run_id, [map_location])
        assert new_attempt != 0
        assert map_location in invalidated
        assert f"N{g.data.output_idx()}" in invalidated
        assert f"N{map_node_index}.M1.N{body_task_index}" not in invalidated

        await runtime.wait_for(run_id, new_attempt, timeout=30)
        assert await runtime.get_outputs(run_id, new_attempt) == [6, 8, 10]
        assert await runtime.get_outputs(run_id) == [6, 8, 10]
