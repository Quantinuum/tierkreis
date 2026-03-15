from pathlib import Path
from uuid import UUID

import pytest

from tests.workers.failing_worker.stubs import exit_code_1, fail, wont_fail
from tierkreis.builder import Graph, Workflow
from tierkreis.controller import run_graph
from tierkreis.controller.data.core import EmptyModel
from tierkreis.controller.data.location import Loc
from tierkreis.controller.data.models import TKR
from tierkreis.controller.executor.uv_executor import UvExecutor
from tierkreis.controller.storage.filestorage import ControllerFileStorage
from tierkreis.exceptions import TierkreisError

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


def test_raise_error() -> None:
    g = will_fail_graph()
    storage = ControllerFileStorage(UUID(int=42), name="will_fail")
    executor = UvExecutor(WORKER_PATH, logs_path=storage.logs_path)
    storage.clean_graph_files()
    with pytest.raises(TierkreisError):
        run_graph(storage, executor, g.data, {}, n_iterations=1000)
    assert storage.node_has_error(Loc("-.N0"))


def test_raises_no_error() -> None:
    g = wont_fail_graph()
    storage = ControllerFileStorage(UUID(int=43), name="wont_fail")
    executor = UvExecutor(WORKER_PATH, logs_path=storage.logs_path)
    storage.clean_graph_files()
    run_graph(storage, executor, g.data, {}, n_iterations=100)
    assert not storage.node_has_error(Loc("-.N0"))


def test_nested_error() -> None:
    g = fail_in_eval()
    storage = ControllerFileStorage(UUID(int=44), name="eval_will_fail")
    executor = UvExecutor(WORKER_PATH, logs_path=storage.logs_path)
    storage.clean_graph_files()
    with pytest.raises(TierkreisError):
        run_graph(storage, executor, g.data, {}, n_iterations=1000)
    assert (storage.logs_path.parent / "-/_error").exists()


def test_non_zero_exit_code() -> None:
    g = non_zero_exit_code()
    storage = ControllerFileStorage(UUID(int=46), name="non_zero_exit_code")
    executor = UvExecutor(WORKER_PATH, logs_path=storage.logs_path)
    storage.clean_graph_files()
    with pytest.raises(TierkreisError):
        run_graph(storage, executor, g.data, {}, n_iterations=1000)
    assert (storage.logs_path.parent / "-/_error").exists()
