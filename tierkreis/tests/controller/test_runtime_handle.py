from pathlib import Path

import pytest

from tests.controller.sample_graphdata import simple_ifelse
from tierkreis import Runtime


def test_uploaded_workflow_can_back_multiple_runs() -> None:
    with Runtime() as runtime:
        workflow_id = runtime.upload_workflow("ifelse", simple_ifelse())

        true_run = runtime.start_workflow(workflow_id, {"pred": True})
        false_run = runtime.start_workflow(workflow_id, {"pred": False})

        assert runtime.wait(true_run, timeout=10) == "Completed"
        assert runtime.wait(false_run, timeout=10) == "Completed"
        assert runtime.get_outputs(true_run) == 1
        assert runtime.get_outputs(false_run) == 2

        true_state = runtime.get_workflow_state(true_run)
        false_state = runtime.get_workflow_state(false_run)
        assert true_state.workflow_id == false_state.workflow_id == workflow_id
        assert true_state.run_id != false_state.run_id
        assert true_state.nodes["N3"].condition is True
        assert false_state.nodes["N3"].condition is False


def test_sqlite_state_survives_reopening(tmp_path: Path) -> None:
    database = tmp_path / "runtime.sqlite"
    assets = tmp_path / "assets"

    with Runtime.sqlite(database, assets) as runtime:
        workflow_id = runtime.upload_workflow("persistent", simple_ifelse())
        run_id = runtime.start_workflow(workflow_id, {"pred": True})
        assert runtime.wait(run_id, timeout=10) == "Completed"
        assert runtime.get_outputs(run_id) == 1

    with Runtime.sqlite(database, assets) as runtime:
        assert runtime.status(run_id) == "Completed"
        assert runtime.get_outputs(run_id) == 1
        state = runtime.get_workflow_state(run_id)
        assert state.workflow_id == workflow_id
        assert state.name == "persistent"


def test_outputs_are_unavailable_before_completion() -> None:
    with Runtime() as runtime:
        workflow_id = runtime.upload_workflow("ifelse", simple_ifelse())
        run_id = runtime.start_workflow(workflow_id, {"pred": True})
        # The graph is small and may already have completed; exercise the API contract
        # only if its state can still be observed in flight.
        if runtime.status(run_id) != "Completed":
            with pytest.raises(ValueError, match="has not finished"):
                runtime.get_outputs(run_id)
        assert runtime.wait(run_id, timeout=10) == "Completed"
