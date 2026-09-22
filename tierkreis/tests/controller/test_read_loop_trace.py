import pytest
from tierkreis._tierkreis import new_default

from tests.controller.loop_graphdata import loop_multiple_acc, loop_multiple_acc_untyped
from tierkreis.controller.data.graph import GraphData

return_value = [
    {"acc1": x, "acc2": y, "acc3": z}
    for x, y, z in zip(range(1, 7), range(2, 13, 2), range(3, 19, 3), strict=True)
]

params: list[tuple[GraphData, list[dict[str, int]], str]] = [
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


@pytest.mark.asyncio
@pytest.mark.parametrize(("graph", "output", "name"), params, ids=ids)
async def test_read_loop_trace_by_name(
    graph: GraphData,
    output: list[dict[str, int]],
    name: str,
) -> None:
    runtime = await new_default()
    with runtime:
        workflow_id = await runtime.save_workflow(name, graph)
        run_id = await runtime.start_new_run(workflow_id, {})
        await runtime.wait_for(run_id, timeout=30)
        actual_output = await runtime.get_loop_iterations(
            run_id, attempt=0, name="my_loop"
        )

    assert actual_output == output
