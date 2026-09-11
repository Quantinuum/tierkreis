import pytest
from tierkreis._tierkreis import new_default, new_in_memory

from tests.controller.sample_graphdata import (
    simple_eagerifelse,
    simple_ifelse,
)
from tierkreis.controller.data.graph import GraphData
from tierkreis.controller.data.types import PType


def eagerifelse_long_running() -> GraphData:
    g = GraphData()
    pred = g.input("pred")
    pred_long = g.func("sleep_worker.sleep_and_return", {"output": pred})("value")

    one = g.const(1)
    one_long = g.func("sleep_worker.sleep_and_return", {"output": one})("value")

    two = g.const(2)
    out = g.eager_if_else(pred_long, one_long, two)("value")
    g.output({"simple_eagerifelse_output": out})
    return g


params = [({"pred": True}, 1), ({"pred": False}, 2)]


@pytest.mark.asyncio
@pytest.mark.parametrize(("inputs", "output"), params)
async def test_eagerifelse_long_running(inputs: dict[str, PType], output: int) -> None:
    graph = eagerifelse_long_running()
    runtime = await new_default()
    with runtime:
        workflow_id = await runtime.save_workflow("eagerifelse_long_running", graph)
        run_id = await runtime.start_new_run(workflow_id, inputs)
        await runtime.wait_for(run_id, 0)
        actual_output = await runtime.get_outputs(run_id, 0)

    assert actual_output == {"simple_eagerifelse_output": output}


@pytest.mark.asyncio
async def test_eagerifelse_nodes() -> None:
    graph = simple_eagerifelse()
    runtime = await new_in_memory()
    with runtime:
        workflow_id = await runtime.save_workflow("simple_eagerifelse", graph)
        run_id = await runtime.start_new_run(workflow_id, {"pred": b"true"})
        await runtime.wait_for(run_id, 0)

    states = await runtime.debug_read_node_states(run_id, 0, ["N3", "N4"])
    assert states["N3"].status == "Complete"
    assert states["N4"].status == "Complete"


@pytest.mark.asyncio
async def test_ifelse_nodes() -> None:
    graph = simple_ifelse()
    runtime = await new_in_memory()
    with runtime:
        workflow_id = await runtime.save_workflow("simple_ifelse", graph)
        run_id = await runtime.start_new_run(workflow_id, {"pred": b"true"})
        await runtime.wait_for(run_id, 0)

    states = await runtime.debug_read_node_states(run_id, 0, ["N1", "N2"])
    assert states["N1"].status == "Complete"
    assert states["N2"].status != "Complete"
