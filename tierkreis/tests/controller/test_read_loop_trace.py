from typing import Awaitable, Callable

import pytest
from tierkreis._tierkreis import new_default, new_in_memory, new_sqlite_memory, Runtime

from tests.controller.loop_graphdata import loop_multiple_acc, loop_multiple_acc_untyped
from tierkreis.controller.data.graph import GraphData

return_value = {
    "acc1": [x for x in range(1, 7)],
    "acc2": [x for x in range(2, 13, 2)],
    "acc3": [x for x in range(3, 19, 3)],
}

params: list[tuple[GraphData, dict[str, list[int]], str]] = [
    (
        loop_multiple_acc_untyped(),
        return_value,
        "multi_acc",
    ),
    (
        loop_multiple_acc().data,
        return_value,
        "multi_acc",
    ),
]
ids = [
    "loop_multiple_acc_untyped",
    "loop_multiple_acc",
]

runtime_fns = [new_default, new_in_memory, new_sqlite_memory]
runtime_fn_ids = ["default", "in_memory", "sqlite_memory"]


@pytest.mark.asyncio
@pytest.mark.parametrize("runtime_fn", runtime_fns, ids=runtime_fn_ids)
@pytest.mark.parametrize(("graph", "output", "name"), params, ids=ids)
async def test_read_loop_trace_by_name(
    runtime_fn: Callable[[], Awaitable[Runtime]],
    graph: GraphData,
    output: dict[str, list[int]],
    name: str,
) -> None:
    runtime = await runtime_fn()
    with runtime:
        workflow_id = await runtime.save_workflow(name, graph)
        run_id = await runtime.start_new_run(workflow_id, {})
        await runtime.wait_for(run_id, timeout=30)
        actual_output = await runtime.get_loop_iterations(
            run_id, attempt=0, name="my_loop"
        )

    assert actual_output == output


@pytest.mark.asyncio
@pytest.mark.parametrize("runtime_fn", runtime_fns, ids=runtime_fn_ids)
@pytest.mark.parametrize(("graph", "output", "name"), params, ids=ids)
async def test_read_loop_trace_by_name_select_port(
    runtime_fn: Callable[[], Awaitable[Runtime]],
    graph: GraphData,
    output: dict[str, list[int]],
    name: str,
) -> None:
    runtime = await runtime_fn()
    with runtime:
        workflow_id = await runtime.save_workflow(name, graph)
        run_id = await runtime.start_new_run(workflow_id, {})
        await runtime.wait_for(run_id, timeout=30)
        actual_output = await runtime.get_loop_iterations(
            run_id,
            attempt=0,
            name="my_loop",
            output_name="acc3",
        )

    assert actual_output == {"acc3": output["acc3"]}
