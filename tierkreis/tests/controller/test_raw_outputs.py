import pytest

from tierkreis.builder import Graph
from tierkreis.controller.data.models import TKR
from tierkreis.runtime import new_in_memory


@pytest.mark.asyncio
async def test_get_raw_outputs() -> None:
    graph = Graph(TKR[int], TKR[int])
    workflow = graph.finish_with_outputs(graph.inputs)

    runtime = await new_in_memory()
    with runtime:
        workflow_id = await runtime.save_workflow("raw outputs", workflow)
        run_id = await runtime.start_new_run(workflow_id, 3)
        await runtime.wait_for(run_id, timeout=30)
        outputs = await runtime.get_raw_outputs(run_id)

    assert outputs == {"value": b"3"}
