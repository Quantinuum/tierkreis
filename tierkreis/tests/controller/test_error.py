from pathlib import Path

import pytest
from tierkreis._tierkreis import new_default

from tests.workers.failing_worker.stubs import exit_code_1, fail, wont_fail
from tierkreis.builder import Graph
from tierkreis.controller.data.core import EmptyModel
from tierkreis.models import TKR, Workflow

WORKER_PATH = Path(__file__).parent.parent / "workers"


def will_fail_graph() -> Workflow[EmptyModel, TKR[int]]:
    graph = Graph(EmptyModel, TKR[int])
    return graph.finish_with_outputs(graph.task(fail()))


def wont_fail_graph() -> Workflow[EmptyModel, TKR[int]]:
    graph = Graph(EmptyModel, TKR[int])
    return graph.finish_with_outputs(graph.task(wont_fail()))


def fail_in_eval() -> Workflow[EmptyModel, TKR[int]]:
    graph = Graph(EmptyModel, TKR[int])
    return graph.finish_with_outputs(graph.eval(will_fail_graph(), EmptyModel()))


def non_zero_exit_code() -> Workflow[EmptyModel, TKR[int]]:
    graph = Graph(EmptyModel, TKR[int])
    return graph.finish_with_outputs(graph.task(exit_code_1()))


@pytest.mark.asyncio
async def test_raise_error() -> None:
    graph = will_fail_graph()
    runtime = await new_default()
    with runtime:
        workflow_id = await runtime.save_workflow("will_fail", graph)
        run_id = await runtime.start_new_run(workflow_id, {})
        with pytest.raises(ValueError) as raises:
            await runtime.wait_for(run_id, 0)

        assert str(raises.value) == "Workflow failed"

    states = await runtime.debug_read_node_states(run_id, 0, ["N0"])
    assert states["N0"].status == "Error"


@pytest.mark.asyncio
async def test_raises_no_error() -> None:
    graph = wont_fail_graph()
    runtime = await new_default()
    with runtime:
        workflow_id = await runtime.save_workflow("wont_fail", graph)
        run_id = await runtime.start_new_run(workflow_id, {})
        await runtime.wait_for(run_id, 0)

    states = await runtime.debug_read_node_states(run_id, 0, ["N0"])
    assert states["N0"].status == "Complete"


@pytest.mark.asyncio
async def test_nested_error() -> None:
    graph = fail_in_eval()
    runtime = await new_default()
    with runtime:
        workflow_id = await runtime.save_workflow("eval_will_fail", graph)
        run_id = await runtime.start_new_run(workflow_id, {})
        with pytest.raises(ValueError):
            await runtime.wait_for(run_id, 0)

    states = await runtime.debug_read_node_states(run_id, 0, ["N1.N0"])
    assert states["N1.N0"].status == "Error"


@pytest.mark.asyncio
async def test_non_zero_exit_code() -> None:
    graph = non_zero_exit_code()
    runtime = await new_default()
    with runtime:
        workflow_id = await runtime.save_workflow("non_zero_exit_code", graph)
        run_id = await runtime.start_new_run(workflow_id, {})
        with pytest.raises(ValueError):
            await runtime.wait_for(run_id, 0)

    states = await runtime.debug_read_node_states(run_id, 0, ["N0"])
    assert states["N0"].status == "Error"
