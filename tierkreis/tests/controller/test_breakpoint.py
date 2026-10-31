from collections.abc import Awaitable, Callable

import pytest
from pytket._tket.circuit import Circuit
from pytket_worker import n_qubits
from tierkreis._tierkreis import Runtime, new_default, new_in_memory, new_sqlite_memory

from tierkreis.builder import Graph
from tierkreis.builtins import iadd
from tierkreis.controller.data.models import TKR
from tierkreis.controller.data.types import Workflow


def breakpoint_graph() -> Workflow[TKR[Circuit], TKR[int]]:
    g = Graph(TKR[Circuit], TKR[int])
    test = g.const(5)
    nq = g.task(n_qubits(g.inputs), has_breakpoint=True, executor="subprocess")  # type: ignore
    out = g.task(iadd(test, nq), has_breakpoint=True)
    return g.finish_with_outputs(out)


runtime_fns = [new_default, new_in_memory, new_sqlite_memory]
runtime_fn_ids = ["default", "in_memory", "sqlite_memory"]


@pytest.mark.asyncio
@pytest.mark.parametrize("runtime_fn", runtime_fns, ids=runtime_fn_ids)
async def test_breakpoint(
    runtime_fn: Callable[[], Awaitable[Runtime]],
) -> None:
    graph = breakpoint_graph()

    runtime = await runtime_fn()
    with runtime:
        workflow_id = await runtime.save_workflow("breakpoints", graph)
        run_id = await runtime.start_new_run(
            workflow_id, {"value": Circuit(2).to_dict()}
        )
        await runtime.wait_for(run_id, timeout=30)

        summary = await runtime.get_summary(run_id)
        assert len(summary.paused_locations) > 0
        assert summary.paused_locations == ["N2"]

        assert await runtime.unpause_nodes(run_id) == ["N2"]
        await runtime.wait_for(run_id, timeout=30)

        summary = await runtime.get_summary(run_id)
        assert len(summary.paused_locations) > 0
        assert summary.paused_locations == ["N3"]

        assert await runtime.unpause_nodes(run_id) == ["N3"]
        await runtime.wait_for(run_id, timeout=30)
        outputs = await runtime.get_outputs(run_id)
        assert outputs == 7
