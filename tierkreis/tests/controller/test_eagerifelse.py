import json
import pytest
from pathlib import Path
from uuid import UUID

from tests.controller.sample_graphdata import (
    simple_eagerifelse,
    simple_ifelse,
)

from tierkreis.controller import run_graph
from tierkreis.controller.data.location import Loc
from tierkreis.controller.data.types import PType
from tierkreis.controller.executor.shell_executor import ShellExecutor
from tierkreis.controller.executor.uv_executor import UvExecutor
from tierkreis.controller.storage.filestorage import ControllerFileStorage
from tierkreis.controller.data.graph import GraphData


def eagerifelse_long_running() -> GraphData:
    g = GraphData()
    pred = g.input("pred")
    pred_long = g.func("controller.sleep_and_return", {"output": pred})("value")

    one = g.const(1)
    one_long = g.func("controller.sleep_and_return", {"output": one})("value")

    two = g.const(2)
    out = g.eager_if_else(pred_long, one_long, two)("value")
    g.output({"simple_eagerifelse_output": out})
    return g


params = [({"pred": True}, 1), ({"pred": False}, 2)]


@pytest.mark.parametrize("input, output", params)
def test_eagerifelse_long_running(input: dict[str, PType], output: int) -> None:
    g = eagerifelse_long_running()
    storage = ControllerFileStorage(UUID(int=150), name="eagerifelse_long_running")

    registry_path = Path(__file__).parent.parent
    executor = UvExecutor(registry_path=registry_path, logs_path=storage.logs_path)

    storage.clean_graph_files()
    run_graph(storage, executor, g, input, n_iterations=20000)
    actual_output = json.loads(storage.read_output(Loc(), "simple_eagerifelse_output"))
    assert actual_output == output


def test_eagerifelse_nodes() -> None:
    g = simple_eagerifelse()
    storage = ControllerFileStorage(UUID(int=151), name="simple_if_else")
    executor = ShellExecutor(Path("./python/examples/launchers"), storage.workflow_dir)
    storage.clean_graph_files()
    run_graph(storage, executor, g, {"pred": b"true"})
    assert storage.is_node_finished(Loc("-.N3"))
    assert storage.is_node_finished(Loc("-.N4"))


def test_ifelse_nodes():
    g = simple_ifelse()
    storage = ControllerFileStorage(UUID(int=152), name="simple_if_else")
    executor = ShellExecutor(Path("./python/examples/launchers"), storage.workflow_dir)
    storage.clean_graph_files()
    run_graph(storage, executor, g, {"pred": b"true"})
    assert storage.is_node_finished(Loc("-.N1"))
    assert not storage.is_node_finished(Loc("-.N2"))
