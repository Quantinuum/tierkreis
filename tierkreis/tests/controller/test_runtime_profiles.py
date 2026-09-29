import os
from pathlib import Path

import pytest
from tierkreis._tierkreis import new_default

from tests.workers.env_worker.stubs import read_env
from tierkreis.builder import ExecutionContext, Graph
from tierkreis.builtins import iadd, sum_list
from tierkreis.models import TKR, Workflow

FIXTURE_ROOT = Path(__file__).parent.parent / "fixtures" / "resource_profiles"
PROFILE_ENV_VAR = "TKR_PROFILE_MARKER"


def nested_builtin_map() -> Workflow[TKR[list[int]], TKR[int]]:
    body = Graph(TKR[int], TKR[int])  # N3.M0.N0
    added = body.task(iadd(a=body.inputs, b=body.const(1)))  # N3.M0.N2, N3.M0.N1
    body_workflow = body.finish_with_outputs(added)  # N3.M0.N3

    graph = Graph(TKR[list[int]], TKR[int])  # N0
    with ExecutionContext("cpu_only"):
        mapped = graph.map(body_workflow, graph.inputs)  # N3
    summed = graph.task(sum_list(mapped), context="subprocess")  # type:ignore N5
    return graph.finish_with_outputs(summed)  # type:ignore N6


def read_profile_env() -> Workflow[TKR[str], TKR[str]]:
    graph = Graph(TKR[str], TKR[str])
    value = graph.task(read_env(name=graph.inputs), context="subprocess")
    return graph.finish_with_outputs(value)


@pytest.fixture
def runtime_e2e_profile(monkeypatch: pytest.MonkeyPatch) -> None:
    monkeypatch.setenv("TKR_RESOURCE_PATH", os.fspath(FIXTURE_ROOT))
    monkeypatch.delenv("TIERKREIS_ENV", raising=False)
    monkeypatch.delenv(PROFILE_ENV_VAR, raising=False)


profiles = ["runtime_e2e", "runtime_e2e2"]


@pytest.mark.asyncio
@pytest.mark.usefixtures("runtime_e2e_profile")
@pytest.mark.parametrize("profile", profiles)
async def test_nested_profile_routes_map_tasks_to_memory(profile: str) -> None:
    runtime = await new_default(profile)
    with runtime:
        workflow_id = await runtime.save_workflow(
            "nested_profile", nested_builtin_map()
        )
        run_id = await runtime.start_new_run(workflow_id, {"value": [1, 2, 3]})
        await runtime.wait_for(run_id, timeout=10)
        outputs = await runtime.get_outputs(run_id)
        states = await runtime.debug_read_node_states(
            run_id,
            0,
            [
                "N3.M0.N2",
                "N3.M1.N2",
                "N3.M2.N2",
                "N5",
            ],
        )
    assert {
        "N3.M0.N2": ("Complete", "cpu_only"),
        "N3.M1.N2": ("Complete", "cpu_only"),
        "N3.M2.N2": ("Complete", "cpu_only"),
        "N5": ("Complete", "subprocess"),
    } == {k: (state.status, state.execution_context) for k, state in states.items()}
    assert outputs == 9


@pytest.mark.asyncio
@pytest.mark.usefixtures("runtime_e2e_profile")
@pytest.mark.parametrize("profile", profiles)
async def test_profile_env_reaches_subprocess_worker(profile: str) -> None:
    runtime = await new_default(profile)
    with runtime:
        workflow_id = await runtime.save_workflow("profile_env", read_profile_env())
        run_id = await runtime.start_new_run(workflow_id, {"value": PROFILE_ENV_VAR})
        await runtime.wait_for(run_id, timeout=10)
        outputs = await runtime.get_outputs(run_id)
        states = await runtime.debug_read_node_states(run_id, 0, ["N1"])

    # Only set via the profile's [env] table, so this also proves the subprocess executor ran it.
    assert outputs == f"from-{profile.replace('_', '-')}-profile"
    assert states["N1"].execution_context == "subprocess"
    assert PROFILE_ENV_VAR not in os.environ
