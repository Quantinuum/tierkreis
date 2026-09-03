import json
from pathlib import Path
from uuid import UUID

import pytest

from tests.controller.sample_graphdata import (
    simple_eagerifelse,
    simple_ifelse,
)
from tierkreis import Runtime
from tierkreis.controller import run_graph
from tierkreis.controller.data.graph import GraphData
from tierkreis.controller.data.location import Loc
from tierkreis.controller.data.types import PType
from tierkreis.controller.executor.uv_executor import UvExecutor
from tierkreis.controller.storage.filestorage import ControllerFileStorage


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


@pytest.mark.parametrize(("inputs", "output"), params)
def test_eagerifelse_long_running(inputs: dict[str, PType], output: int) -> None:
    g = eagerifelse_long_running()
    storage = ControllerFileStorage(UUID(int=150), name="eagerifelse_long_running")

    registry_path = Path(__file__).parent.parent / "workers"
    executor = UvExecutor(registry_path=registry_path, logs_path=storage.logs_path)

    storage.clean_graph_files()
    run_graph(storage, executor, g, inputs, n_iterations=20000)
    actual_output = json.loads(storage.read_output(Loc(), "simple_eagerifelse_output"))
    assert actual_output == output


def test_eagerifelse_nodes() -> None:
    g = simple_eagerifelse()
    with Runtime() as runtime:
        workflow_id = runtime.upload_workflow("simple_eager_if_else", g)
        run_id = runtime.start_workflow(workflow_id, {"pred": True})
        assert runtime.wait(run_id, timeout=30) == "Completed"
        state = runtime.get_workflow_state(run_id)
        assert state.nodes["N3"].status == "Completed"
        assert state.nodes["N4"].status == "Completed"


def test_ifelse_nodes() -> None:
    g = simple_ifelse()
    with Runtime() as runtime:
        workflow_id = runtime.upload_workflow("simple_if_else", g)
        run_id = runtime.start_workflow(workflow_id, {"pred": True})
        assert runtime.wait(run_id, timeout=30) == "Completed"
        state = runtime.get_workflow_state(run_id)
        assert state.nodes["N1"].status == "Completed"
        assert "N2" not in state.nodes
