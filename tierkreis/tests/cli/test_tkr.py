import json
import pytest
import sys
from pathlib import Path
from unittest import mock
from uuid import UUID

from tierkreis.cli.tkr import load_graph, _load_inputs, main
from tierkreis.controller.data.graph import GraphData
from tierkreis.controller.data.types import PType
from tierkreis.exceptions import TierkreisError

from tests.controller.sample_graphdata import simple_eval

simple_eval_graph = simple_eval()

graph_params = [
    ("tests.controller.sample_graphdata:simple_eval", simple_eval_graph),
    ("tierkreis/tests/controller/sample_graphdata.py:simple_eval", simple_eval_graph),
]


@pytest.mark.parametrize("input,graph", graph_params, ids=["load_module", "load_file"])
def test_load_graph(input: str, graph: GraphData) -> None:
    assert load_graph(input) == graph


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
    "input,result", input_params, ids=["json_input", "binary_input"]
)
def test_load_inputs(input: list[str], result: dict[str, PType]) -> None:
    assert _load_inputs(input) == result


def test_load_inputs_invalid() -> None:
    with pytest.raises(FileNotFoundError):
        _load_inputs(["data.json"])
    with pytest.raises(FileNotFoundError):
        _load_inputs(["at_least_a:colon"])
    with pytest.raises(TierkreisError):
        _load_inputs(["wrong_format"])


default_args = [
    "tkr",
    "run",
    "--run-id",
    "1860",
    "-v",
    "-o",
    "-n",
    "500",
    "-p",
    "0.02",
    "-r",
    "--name",
    "test_name",
    "--uv",
    "--registry-path",
    "tests/controller/sample_graphdata",
]

cli_params = [
    (
        default_args + ["-f", "tierkreis/tests/cli/data/sample_graph"],
        {"simple_eval_output": 12},
    ),
    (
        default_args
        + [
            "-g",
            "tests.controller.sample_graphdata:factorial",
            "-i",
            "n:tierkreis/tests/cli/data/n",
            "factorial:tierkreis/tests/cli/data/factorial",
        ],
        {"factorial_output": 120},
    ),
]


@pytest.mark.parametrize(
    "args,result", cli_params, ids=["simple_eval_cli", "factorial_cli"]
)
def test_end_to_end(args: list[str], result: dict[str, bytes]) -> None:
    with mock.patch.object(sys, "argv", args):
        main()
    for key, value in result.items():
        with open(
            Path.home()
            / ".tierkreis"
            / "checkpoints"
            / str(UUID(int=1860))
            / f"-/outputs/{key}",
            "rb",
        ) as fh:
            c = json.loads(fh.read())
        assert c == value
