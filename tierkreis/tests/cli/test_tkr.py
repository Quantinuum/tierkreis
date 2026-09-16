import sys
from pathlib import Path
from unittest import mock

import pytest

from tests.controller.sample_graphdata import simple_eval
from tierkreis.cli.run import _load_inputs, load_graph
from tierkreis.cli.tkr import main
from tierkreis.controller.data.graph import GraphData
from tierkreis.controller.data.types import PType
from tierkreis.exceptions import TierkreisError

simple_eval_graph = simple_eval()

graph_params = [
    ("tests.controller.sample_graphdata:simple_eval", simple_eval_graph),
    ("tierkreis/tests/controller/sample_graphdata.py:simple_eval", simple_eval_graph),
]


@pytest.mark.parametrize(
    ("inputs", "graph"),
    graph_params,
    ids=["load_module", "load_file"],
)
def test_load_graph(inputs: str, graph: GraphData) -> None:
    assert load_graph(inputs) == graph


def test_load_graph_invalid() -> None:
    with pytest.raises(FileNotFoundError):
        load_graph("sample_graphdata.py:simple_eval")
    with pytest.raises(ModuleNotFoundError):
        load_graph("sample_graphdata:simple_eval")
    with pytest.raises(TierkreisError):
        load_graph("invalid_arg")
    with pytest.raises(ModuleNotFoundError):
        load_graph("at_least_a:colon")


input_params = [
    (
        ["tierkreis/tests/cli/data/data.json"],
        {
            "a_string": b'"string"',
            "a_value": b"5",
        },
    ),
    (
        [
            "input1:tierkreis/tests/cli/data/input1",
            "input2:tierkreis/tests/cli/data/input2",
        ],
        {"input1": b"test\n", "input2": {"a": 5, "b": "string"}},
    ),
]


@pytest.mark.parametrize(
    ("inputs", "result"),
    input_params,
    ids=["json_input", "binary_input"],
)
def test_load_inputs(inputs: list[str], result: dict[str, PType]) -> None:
    assert _load_inputs(inputs) == result


def test_load_inputs_invalid() -> None:
    with pytest.raises(FileNotFoundError):
        _load_inputs(["data.json"])
    with pytest.raises(FileNotFoundError):
        _load_inputs(["at_least_a:colon"])
    with pytest.raises(TierkreisError):
        _load_inputs(["wrong_format"])


def test_rust_cli_end_to_end(tmp_path: Path, capfd: pytest.CaptureFixture[str]) -> None:
    project_args = ["tkr", "init", "project", str(tmp_path)]
    with mock.patch.object(sys, "argv", project_args):
        main()

    inputs = tmp_path / "empty_inputs.json"
    inputs.write_text("{}", encoding="utf-8")
    run_args = [
        "tkr",
        "run",
        "tierkreis/tests/cli/data/sample_graph",
        "--inputs",
        str(inputs),
        "--config",
        str(tmp_path / "tierkreis.toml"),
        "--print-output",
    ]
    with mock.patch.object(sys, "argv", run_args):
        main()

    stdout, _ = capfd.readouterr()
    assert '"simple_eval_output": 12' in stdout
    assert (tmp_path / "tkr/workers/example_worker/api/api.py").is_file()
