import os
import sys
from pathlib import Path
from uuid import UUID

import pytest

from tests.workers.failing_worker.stubs import exit_code_1, fail, wont_fail
from tierkreis import Runtime
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


@pytest.fixture
def runtime(tmp_path: Path, monkeypatch: pytest.MonkeyPatch):
    """A subprocess runtime with the failing test worker on PATH."""
    executable = tmp_path / "tkr-failing-worker"
    executable.write_text(
        f"#!{sys.executable}\n"
        "import runpy\n"
        f"runpy.run_path({str(WORKER_PATH / 'failing_worker/main.py')!r}, run_name='__main__')\n"
    )
    executable.chmod(0o755)
    monkeypatch.setenv("PATH", f"{tmp_path}{os.pathsep}{os.environ['PATH']}")
    with Runtime.sqlite(
        tmp_path / "runtime.sqlite",
        tmp_path / "assets",
        executor="subprocess",
    ) as runtime:
        yield runtime


def run(runtime: Runtime, name: str, workflow: Workflow) -> tuple[UUID, str]:
    workflow_id = runtime.upload_workflow(name, workflow)
    run_id = runtime.start_workflow(workflow_id, {})
    return run_id, runtime.wait(run_id, timeout=30)


def test_raise_error(runtime: Runtime) -> None:
    run_id, status = run(runtime, "will_fail", will_fail_graph())
    assert status == "Errored"
    state = runtime.get_workflow_state(run_id)
    errors = [node for node in state.nodes.values() if node.status == "Errored"]
    assert len(errors) == 1
    assert errors[0].error is not None


def test_raises_no_error(runtime: Runtime) -> None:
    run_id, status = run(runtime, "wont_fail", wont_fail_graph())
    state = runtime.get_workflow_state(run_id)
    assert status == "Completed", state.nodes["N0"].error_detail
    assert runtime.get_outputs(run_id) == 0
    assert all(
        node.status != "Errored"
        for node in runtime.get_workflow_state(run_id).nodes.values()
    )


def test_nested_error(runtime: Runtime) -> None:
    run_id, status = run(runtime, "eval_will_fail", fail_in_eval())
    assert status == "Errored"
    state = runtime.get_workflow_state(run_id)
    assert any(node.status == "Errored" for node in state.nodes.values())


def test_non_zero_exit_code(runtime: Runtime) -> None:
    run_id, status = run(runtime, "non_zero_exit_code", non_zero_exit_code())
    assert status == "Errored"
    state = runtime.get_workflow_state(run_id)
    errors = [node for node in state.nodes.values() if node.status == "Errored"]
    assert errors
    assert any("error code" in (node.error or "") for node in errors)
