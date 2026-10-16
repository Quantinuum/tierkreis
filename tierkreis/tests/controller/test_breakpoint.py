from collections.abc import Awaitable, Callable

import pytest
from tierkreis._tierkreis import (
    Runtime,
    WaitUntil,
    new_default,
    new_in_memory,
    new_sqlite_memory,
)

from tierkreis.builder import Graph
from tierkreis.builtins import iadd
from tierkreis.controller.data.models import TKR
from tierkreis.controller.data.types import Workflow

runtime_fns = [new_default, new_in_memory, new_sqlite_memory]
runtime_fn_ids = ["default", "in_memory", "sqlite_memory"]
DIAMOND_LOCS = {"left": "N2", "right": "N4", "joined": "N5"}
NESTED_LOCS = {"M0": "N3.M0.N2", "M1": "N3.M1.N2", "M2": "N3.M2.N2"}


def diamond_graph() -> Workflow[TKR[int], TKR[int]]:
    g = Graph(TKR[int], TKR[int])
    left = g.task(iadd(g.inputs, g.const(1)), has_breakpoint=True)
    right = g.task(iadd(g.inputs, g.const(2)))
    joined = g.task(iadd(left, right), has_breakpoint=True)
    return g.finish_with_outputs(joined)


async def start_run(
    runtime: Runtime,
    name: str,
    graph: Workflow,
    inputs: dict[str, object],
) -> str:
    workflow_id = await runtime.save_workflow(name, graph)
    return await runtime.start_new_run(workflow_id, inputs, enable_breakpoints=True)


async def enter_nested_items(runtime: Runtime, run_id: str) -> list[str]:
    breakpoints = await runtime.wait_for(
        run_id, condition=WaitUntil.Breakpoint, timeout=10
    )
    assert breakpoints == ["N3"]
    await runtime.resume_breakpoints(run_id, ["N3"])
    return await runtime.wait_for(run_id, condition=WaitUntil.Blocked, timeout=10)


@pytest.mark.asyncio
@pytest.mark.parametrize("runtime_fn", runtime_fns, ids=runtime_fn_ids)
async def test_breakpoint_pauses_at_first_node(
    runtime_fn: Callable[[], Awaitable[Runtime]],
) -> None:
    runtime = await runtime_fn()
    with runtime:
        run_id = await start_run(runtime, "breakpoints", diamond_graph(), {"value": 3})
        breakpoints = await runtime.wait_for(
            run_id, condition=WaitUntil.Breakpoint, timeout=10
        )
        assert breakpoints == [DIAMOND_LOCS["left"]]
        breakpoints = await runtime.wait_for(
            run_id, condition=WaitUntil.Blocked, timeout=10
        )
        assert breakpoints == [DIAMOND_LOCS["left"]]
        assert await runtime.paused_nodes(run_id) == [DIAMOND_LOCS["left"]]
        states = await runtime.debug_read_node_states(
            run_id, 0, [DIAMOND_LOCS["left"], DIAMOND_LOCS["right"]]
        )
        assert states[DIAMOND_LOCS["left"]].status == "Paused"
        assert states[DIAMOND_LOCS["right"]].status == "Complete"
        assert (await runtime.get_summary(run_id)).complete_time is None


@pytest.mark.asyncio
@pytest.mark.parametrize("runtime_fn", runtime_fns, ids=runtime_fn_ids)
async def test_invalid_resume(
    runtime_fn: Callable[[], Awaitable[Runtime]],
) -> None:
    runtime = await runtime_fn()
    with runtime:
        run_id = await start_run(
            runtime, "invalid_resume", diamond_graph(), {"value": 3}
        )
        await runtime.wait_for(run_id, condition=WaitUntil.Blocked, timeout=10)
        with pytest.raises(ValueError, match="not resumable|not hit a breakpoint"):
            await runtime.resume_breakpoints(run_id, [DIAMOND_LOCS["left"], "N999"])
        assert await runtime.paused_nodes(run_id) == [DIAMOND_LOCS["left"]]


@pytest.mark.asyncio
@pytest.mark.parametrize("runtime_fn", runtime_fns, ids=runtime_fn_ids)
async def test_empty_resume(
    runtime_fn: Callable[[], Awaitable[Runtime]],
) -> None:
    runtime = await runtime_fn()
    with runtime:
        run_id = await start_run(runtime, "empty_resume", diamond_graph(), {"value": 3})
        await runtime.wait_for(run_id, condition=WaitUntil.Blocked, timeout=10)
        assert await runtime.resume_breakpoints(run_id, []) == []
        assert await runtime.paused_nodes(run_id) == [DIAMOND_LOCS["left"]]


@pytest.mark.asyncio
@pytest.mark.parametrize("runtime_fn", runtime_fns, ids=runtime_fn_ids)
async def test_resuming_reaches_next(
    runtime_fn: Callable[[], Awaitable[Runtime]],
) -> None:
    runtime = await runtime_fn()
    with runtime:
        run_id = await start_run(
            runtime, "next_breakpoint", diamond_graph(), {"value": 3}
        )
        await runtime.wait_for(run_id, condition=WaitUntil.Blocked, timeout=10)
        assert await runtime.resume_breakpoints(run_id, [DIAMOND_LOCS["left"]]) == [
            DIAMOND_LOCS["left"]
        ]
        breakpoints = await runtime.wait_for(
            run_id, condition=WaitUntil.Breakpoint, timeout=10
        )
        assert breakpoints == [DIAMOND_LOCS["joined"]]


@pytest.mark.asyncio
@pytest.mark.parametrize("runtime_fn", runtime_fns, ids=runtime_fn_ids)
async def test_resuming_all_breakpoints(
    runtime_fn: Callable[[], Awaitable[Runtime]],
) -> None:
    runtime = await runtime_fn()
    with runtime:
        run_id = await start_run(
            runtime, "complete_diamond", diamond_graph(), {"value": 3}
        )
        await runtime.wait_for(run_id, condition=WaitUntil.Blocked, timeout=10)
        await runtime.resume_breakpoints(run_id, [DIAMOND_LOCS["left"]])
        await runtime.wait_for(run_id, condition=WaitUntil.Breakpoint, timeout=10)
        assert await runtime.resume_breakpoints(run_id) == [DIAMOND_LOCS["joined"]]
        breakpoints = await runtime.wait_for(
            run_id, condition=WaitUntil.Blocked, timeout=10
        )
        assert breakpoints == []
        assert await runtime.wait_for(run_id, timeout=10) == []
        assert await runtime.get_outputs(run_id) == 9
        assert await runtime.paused_nodes(run_id) == []


def nested_breakpoints() -> Workflow[TKR[list[int]], TKR[list[int]]]:
    inner = Graph(TKR[int], TKR[int])
    inner_task = inner.task(iadd(inner.inputs, inner.const(1)), has_breakpoint=True)
    body = inner.finish_with_outputs(inner_task)
    outer = Graph(TKR[list[int]], TKR[list[int]])
    mapped = outer.map(body, outer.inputs, has_breakpoint=True)
    return outer.finish_with_outputs(mapped)


@pytest.mark.asyncio
@pytest.mark.parametrize("runtime_fn", runtime_fns, ids=runtime_fn_ids)
async def test_nested_breakpoints(
    runtime_fn: Callable[[], Awaitable[Runtime]],
) -> None:
    runtime = await runtime_fn()
    with runtime:
        run_id = await start_run(
            runtime, "map_breakpoints", nested_breakpoints(), {"value": [0, 1, 2]}
        )
        assert await enter_nested_items(runtime, run_id) == list(NESTED_LOCS.values())


@pytest.mark.asyncio
@pytest.mark.parametrize("runtime_fn", runtime_fns, ids=runtime_fn_ids)
async def test_resuming_one_nested(
    runtime_fn: Callable[[], Awaitable[Runtime]],
) -> None:
    runtime = await runtime_fn()
    with runtime:
        run_id = await start_run(
            runtime, "resume_one_nested", nested_breakpoints(), {"value": [0, 1, 2]}
        )
        assert await enter_nested_items(runtime, run_id) == list(NESTED_LOCS.values())
        await runtime.resume_breakpoints(run_id, [NESTED_LOCS["M0"]])
        breakpoints = await runtime.wait_for(
            run_id, condition=WaitUntil.Blocked, timeout=10
        )
        assert breakpoints == [NESTED_LOCS["M1"], NESTED_LOCS["M2"]]
        states = await runtime.debug_read_node_states(
            run_id, 0, list(NESTED_LOCS.values())
        )
        assert states[NESTED_LOCS["M0"]].status == "Complete"
        assert states[NESTED_LOCS["M1"]].status == "Paused"
        assert states[NESTED_LOCS["M2"]].status == "Paused"
        assert await runtime.paused_nodes(run_id) == [
            NESTED_LOCS["M1"],
            NESTED_LOCS["M2"],
        ]


@pytest.mark.asyncio
@pytest.mark.parametrize("runtime_fn", runtime_fns, ids=runtime_fn_ids)
async def test_nested_breakpoints_unpaused(
    runtime_fn: Callable[[], Awaitable[Runtime]],
) -> None:
    runtime = await runtime_fn()
    with runtime:
        run_id = await start_run(
            runtime, "map_breakpoints_alt", nested_breakpoints(), {"value": [0, 1, 2]}
        )
        assert await enter_nested_items(runtime, run_id) == list(NESTED_LOCS.values())
        await runtime.resume_breakpoints(run_id, [NESTED_LOCS["M0"]])
        breakpoints = await runtime.wait_for(
            run_id, condition=WaitUntil.Breakpoint, timeout=10
        )
        assert breakpoints == [NESTED_LOCS["M1"], NESTED_LOCS["M2"]]
        states = await runtime.debug_read_node_states(
            run_id, 0, list(NESTED_LOCS.values())
        )
        assert states[NESTED_LOCS["M0"]].status != "Paused"
        assert states[NESTED_LOCS["M1"]].status == "Paused"
        assert states[NESTED_LOCS["M2"]].status == "Paused"
        assert await runtime.paused_nodes(run_id) == [
            NESTED_LOCS["M1"],
            NESTED_LOCS["M2"],
        ]


@pytest.mark.asyncio
@pytest.mark.parametrize("runtime_fn", runtime_fns, ids=runtime_fn_ids)
async def test_resuming_all_nested(
    runtime_fn: Callable[[], Awaitable[Runtime]],
) -> None:
    runtime = await runtime_fn()
    with runtime:
        run_id = await start_run(
            runtime, "complete_nested", nested_breakpoints(), {"value": [0, 1, 2]}
        )
        await enter_nested_items(runtime, run_id)
        await runtime.resume_breakpoints(run_id)
        breakpoints = await runtime.wait_for(
            run_id, condition=WaitUntil.Blocked, timeout=10
        )
        assert breakpoints == []
        assert await runtime.get_outputs(run_id) == [1, 2, 3]
