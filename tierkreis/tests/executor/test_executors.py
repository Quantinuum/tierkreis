from pathlib import Path
from uuid import UUID

from tierkreis.builder import GraphBuilder
from tierkreis.builtins.stubs import neg
from tierkreis.controller import run_graph
from tierkreis.controller.data.location import Loc
from tierkreis.controller.data.models import TKR
from tierkreis.controller.executor.multiple import MultipleExecutor
from tierkreis.controller.executor.shell_executor import ShellExecutor
from tierkreis.controller.executor.stdinout import StdInOut
from tierkreis.controller.executor.task_executor import TaskExecutor
from tierkreis.controller.executor.uv_executor import UvExecutor
from tierkreis.controller.storage.filestorage import ControllerFileStorage
from tierkreis.storage import read_outputs

from tests.workers.hello_world_worker.stubs import greet


WORKER_PATH = Path(__file__).parent.parent / "workers"


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
        WORKER_PATH,
        workflow_dir=storage.workflow_dir,
        env={"TEST_FLAG": "Hello"},
    )
    storage.clean_graph_files()
    run_graph(storage, executor, g, {"value": "world"})
    actual_output = read_outputs(g, storage)
    assert actual_output == b'Hello "world"\n'
    assert storage._exec_data_path(Loc()).parent.exists()
    node_loc = Loc("-.N1")
    data = storage.read_executor_data()
    assert node_loc in data
    exec_data = data[node_loc]
    assert "TEST_FLAG" in exec_data.env and exec_data.env["TEST_FLAG"] == "Hello"
    assert exec_data.executor == str(executor.__class__)
    assert "main.sh" in exec_data.launch_command


def test_suppress_env():
    g = shell_graph()
    storage = ControllerFileStorage(UUID(int=301), name="Shell")
    executor = ShellExecutor(
        WORKER_PATH,
        workflow_dir=storage.workflow_dir,
        env={"TEST_FLAG": "Hello"},
        log_env_in_debug=False,
    )
    storage.clean_graph_files()
    run_graph(storage, executor, g, {"value": "world"})
    actual_output = read_outputs(g, storage)
    assert actual_output == b'Hello "world"\n'
    assert storage._exec_data_path(Loc()).parent.exists()
    node_loc = Loc("-.N1")
    data = storage.read_executor_data()
    assert node_loc in data
    exec_data = data[node_loc]
    assert "TEST_FLAG" not in exec_data.env
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
    assert storage._exec_data_path(Loc()).parent.exists()
    node_loc = Loc("-.N1")
    data = storage.read_executor_data()
    assert node_loc in data
    exec_data = data[node_loc]
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
    storage = ControllerFileStorage(UUID(int=303), name="UV")
    executor = UvExecutor(
        WORKER_PATH,
        logs_path=storage.logs_path,
        env={"TEST_FLAG": "Hello"},
    )
    storage.clean_graph_files()
    run_graph(storage, executor, g, {"value": "world"})
    actual_output = read_outputs(g, storage)
    assert actual_output == "hello world"
    assert storage._exec_data_path(Loc()).parent.exists()
    node_loc = Loc("-.N2")
    data = storage.read_executor_data()
    assert node_loc in data
    exec_data = data[node_loc]
    assert "TEST_FLAG" in exec_data.env
    assert exec_data.executor == str(executor.__class__)
    assert "uv" in exec_data.launch_command
    assert any("ruff" in x for x in exec_data.packages)


def stdinout_graph():
    g = GraphBuilder(TKR[str], TKR[str])
    result = g.data.func(  # escape hatch into untyped builder
        "stdinout_worker.greet",
        {"greeting": g.inputs.value_ref()},
    )
    output: TKR[str] = TKR(*result("value"))  # unsafe cast

    g.outputs(output)
    return g


def test_stdinout_executor():
    g = stdinout_graph()
    storage = ControllerFileStorage(UUID(int=304), name="StdInOut")
    executor = StdInOut(
        WORKER_PATH,
        workflow_dir=storage.workflow_dir,
    )
    storage.clean_graph_files()
    run_graph(storage, executor, g, {"value": "world"})
    actual_output = read_outputs(g, storage)
    assert actual_output == b'Hello "world"\n'
    assert storage._exec_data_path(Loc()).parent.exists()
    node_loc = Loc("-.N1")
    data = storage.read_executor_data()
    assert node_loc in data
    exec_data = data[node_loc]
    assert "TEST_FLAG" not in exec_data.env
    assert exec_data.executor == str(executor.__class__)
    assert "main.sh" in exec_data.launch_command


def task_graph():
    # Both tasks are the same, we just use different names to test the task executor
    g = GraphBuilder(TKR[str], TKR[str])
    first_call = g.data.func(
        "shell_worker.meet",
        {"greeting": g.inputs.value_ref()},
    )
    out: TKR[str] = TKR(*first_call("value"))
    second_call = g.data.func(
        "shell_worker.greet",
        {"greeting": out.value_ref()},
    )
    output: TKR[str] = TKR(*second_call("value"))
    g.outputs(output)
    return g


def test_task_executor():
    g = task_graph()
    storage = ControllerFileStorage(UUID(int=305), name="Task")
    first = ShellExecutor(
        WORKER_PATH,
        workflow_dir=storage.workflow_dir,
        env={"TEST_FLAG": "cruel"},
    )
    second = ShellExecutor(
        WORKER_PATH,
        workflow_dir=storage.workflow_dir,
        env={"TEST_FLAG": "Goodbye"},
    )
    executor = TaskExecutor(
        {"shell_worker.meet": first, "shell_worker.greet": second}, storage
    )
    storage.clean_graph_files()
    run_graph(storage, executor, g, {"value": "world"})
    actual_output = read_outputs(g, storage)
    assert actual_output == b'Goodbye cruel "world"\n'
    assert storage._exec_data_path(Loc()).parent.exists()
    node_loc = Loc("-.N1")
    data = storage.read_executor_data()
    assert node_loc in data
    exec_data = data[node_loc]
    assert "TEST_FLAG" in exec_data.env and exec_data.env["TEST_FLAG"] == "cruel"
    assert exec_data.executor == f"{executor.__class__}:{str(first.__class__)}"
    assert "main.sh" in exec_data.launch_command
    node_loc = Loc("-.N2")
    exec_data = storage.read_executor_data(node_loc)
    assert "TEST_FLAG" in exec_data.env and exec_data.env["TEST_FLAG"] == "Goodbye"
    assert exec_data.executor == f"{executor.__class__}:{str(second.__class__)}"
    assert "main.sh" in exec_data.launch_command


def multiple_graph():
    # Both tasks are the same, we just use different names to test the task executor
    g = GraphBuilder(TKR[str], TKR[str])
    first_call = g.data.func(
        "shell_worker.meet",
        {"greeting": g.inputs.value_ref()},
    )
    out: TKR[str] = TKR(*first_call("value"))
    second_call = g.data.func(
        "stdinout_worker.greet",
        {"greeting": out.value_ref()},
    )
    output: TKR[str] = TKR(*second_call("value"))
    g.outputs(output)
    return g


def test_multiple_executor():
    g = multiple_graph()
    storage = ControllerFileStorage(UUID(int=306), name="Multiple")
    first = ShellExecutor(
        WORKER_PATH,
        workflow_dir=storage.workflow_dir,
        env={"TEST_FLAG": "beautiful"},
    )
    second = StdInOut(
        WORKER_PATH,
        workflow_dir=storage.workflow_dir,
    )
    executor = MultipleExecutor(
        first, {"second": second}, {"stdinout_worker": "second"}
    )
    storage.clean_graph_files()
    run_graph(storage, executor, g, {"value": "world"})
    actual_output = read_outputs(g, storage)
    assert actual_output == b'Hello beautiful "world"\n'
    assert storage._exec_data_path(Loc()).parent.exists()
    node_loc = Loc("-.N1")
    data = storage.read_executor_data()
    assert node_loc in data
    exec_data = data[node_loc]
    assert "TEST_FLAG" in exec_data.env and exec_data.env["TEST_FLAG"] == "beautiful"
    assert exec_data.executor == f"{executor.__class__}:{str(first.__class__)}"
    assert "main.sh" in exec_data.launch_command
    node_loc = Loc("-.N2")
    exec_data = data[node_loc]
    assert "TEST_FLAG" not in exec_data.env
    assert exec_data.executor == f"{executor.__class__}:{str(second.__class__)}"
    assert "main.sh" in exec_data.launch_command
