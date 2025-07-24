import pytest
import json
import logging
from pathlib import Path
from uuid import UUID
from unittest import mock


from tierkreis.cli.run_workflow import run_workflow
from tests.controller.sample_graphdata import simple_eval
from tierkreis.controller.data.types import ptype_from_bytes
from tierkreis_core import GraphData

logger = logging.getLogger(__name__)


@pytest.fixture()
def graph() -> GraphData:
    return simple_eval()


def test_run_workflow(graph: GraphData) -> None:
    inputs = {}
    run_workflow(inputs=inputs, graph=graph, run_id=31415)
    with open(
        Path.home()
        / ".tierkreis"
        / "checkpoints"
        / str(UUID(int=31415))
        / "-/outputs/simple_eval_output",
        "rb",
    ) as fh:
        c = ptype_from_bytes(fh.read())

    assert c == 12


def test_run_workflow_with_output(graph: GraphData, capfd) -> None:
    inputs = {}
    run_workflow(inputs=inputs, graph=graph, run_id=31415, print_output=True)
    out, _ = capfd.readouterr()
    assert "simple_eval_output: b'12'\n" in out


@mock.patch("uuid.uuid4", return_value=UUID(int=31415))
def test_run_workflow_default_run_id(_, graph: GraphData) -> None:
    inputs = {}
    run_workflow(inputs=inputs, graph=graph)
    with open(
        Path.home()
        / ".tierkreis"
        / "checkpoints"
        / str(UUID(int=31415))
        / "-/outputs/simple_eval_output",
        "rb",
    ) as fh:
        c = json.loads(fh.read())
    assert c == 12


def test_run_workflow_uv_executor(graph: GraphData) -> None:
    inputs = {}
    run_workflow(
        inputs=inputs,
        graph=graph,
        run_id=31415,
        use_uv_worker=True,
        registry_path=Path("."),
    )
