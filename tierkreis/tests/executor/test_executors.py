import json
from pathlib import Path
from uuid import UUID

from docs.source.worker.hello_stubs import greet

from tierkreis.builder import GraphBuilder
from tierkreis.builtins.stubs import neg
from tierkreis.consts import PACKAGE_PATH
from tierkreis.controller import run_graph
from tierkreis.controller.data.location import Loc
from tierkreis.controller.data.models import TKR

# from tierkreis.controller.executor.multiple import MultipleExecutor
from tierkreis.controller.executor.shell_executor import ShellExecutor

# from tierkreis.controller.executor.stdinout import StdInOut
# from tierkreis.controller.executor.task_executor import TaskExecutor
from tierkreis.controller.executor.uv_executor import UvExecutor
from tierkreis.controller.storage.data import ExecutorDebugData
from tierkreis.controller.storage.filestorage import ControllerFileStorage
from tierkreis.storage import read_outputs


def shell_graph():
    g = GraphBuilder(TKR[str], TKR[str])
    result = g.data.func(  # escape hatch into untyped builder
        "shell_worker.greet",
        {"greeting": g.inputs.value_ref()},
    )
    output: TKR[str] = TKR(*result("value"))  # unsafe cast

    g.outputs(output)
    return g


def test_shell_executor():
    g = shell_graph()
    storage = ControllerFileStorage(UUID(int=301), name="Shell")
    executor = ShellExecutor(
        Path(__file__).parent,
        workflow_dir=storage.workflow_dir,
        env={"TEST_FLAG": "Hello"},
    )
    storage.clean_graph_files()
    run_graph(storage, executor, g, {"value": "world"})
    actual_output = read_outputs(g, storage)
    assert actual_output == b'Hello "world"\n'
    assert storage._exec_data_path().exists()
    node_loc = Loc("-.N1")
    with open(storage._exec_data_path(), "r") as fh:
        data = json.loads(fh.read())
    assert node_loc in data
    exec_data = ExecutorDebugData(**data[node_loc])
    assert "TEST_FLAG" in exec_data.env and exec_data.env["TEST_FLAG"] == "Hello"
    assert exec_data.executor == str(executor.__class__)
    assert "main.sh" in exec_data.launch_command


def builtin_graph() -> GraphBuilder[TKR[bool], TKR[bool]]:
    g = GraphBuilder(TKR[bool], TKR[bool])
    g.outputs(g.task(neg(g.inputs)))
    return g


def test_builtin_executor():
    g = builtin_graph()
    storage = ControllerFileStorage(UUID(int=302), name="Shell")
    executor = ShellExecutor(
        Path("./python/examples/launchers"),
        workflow_dir=storage.workflow_dir,
        env={"TEST_FLAG": "Hello"},
    )
    storage.clean_graph_files()
    run_graph(storage, executor, g, {"value": True})
    actual_output = read_outputs(g, storage)
    assert not actual_output
    assert storage._exec_data_path().exists()
    node_loc = Loc("-.N1")
    with open(storage._exec_data_path(), "r") as fh:
        data = json.loads(fh.read())
    assert node_loc in data
    exec_data = ExecutorDebugData(**data[node_loc])
    assert "TEST_FLAG" not in exec_data.env
    assert exec_data.executor == "builtin"


def hello_graph() -> GraphBuilder[TKR[str], TKR[str]]:
    g = GraphBuilder(TKR[str], TKR[str])
    hello = g.const("hello ")
    output = g.task(greet(greeting=hello, subject=g.inputs))
    g.outputs(output)

    return g


def test_uv_executor():
    g = hello_graph()
    storage = ControllerFileStorage(UUID(int=303), name="Shell")
    executor = UvExecutor(
        PACKAGE_PATH.parent / "examples" / "example_workers",
        logs_path=storage.logs_path,
        env={"TEST_FLAG": "Hello"},
    )
    storage.clean_graph_files()
    run_graph(storage, executor, g, {"value": "world"})
    actual_output = read_outputs(g, storage)
    assert actual_output == "hello world"
    assert storage._exec_data_path().exists()
    node_loc = Loc("-.N2")
    with open(storage._exec_data_path(), "r") as fh:
        data = json.loads(fh.read())
    assert node_loc in data
    exec_data = ExecutorDebugData(**data[node_loc])
    assert "TEST_FLAG" in exec_data.env
    assert exec_data.executor == str(executor.__class__)
    assert "uv" in exec_data.launch_command
    assert any("tierkreis" in x for x in exec_data.packages)
