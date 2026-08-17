import json
from pathlib import Path
from unittest import mock
from uuid import UUID

import pytest

from tests.controller.sample_graphdata import simple_eval
from tierkreis.cli.run_workflow import run_workflow
from tierkreis.controller.data.graph import GraphData
from tierkreis.controller.data.types import ptype_from_bytes


@pytest.fixture
def graph() -> GraphData:
    return simple_eval()


def test_run_workflow(graph: GraphData) -> None:
    inputs = {}
    run_workflow(inputs=inputs, workflow=graph, run_id=31415)  # type:ignore
    with Path.open(
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
    run_workflow(inputs=inputs, workflow=graph, run_id=31415, print_output=True)
    out, _ = capfd.readouterr()
    assert "'simple_eval_output': 12" in out


@pytest.fixture
def _patch_uuid4() -> mock.Mock:
    with mock.patch("uuid.uuid4", return_value=UUID(int=31415)) as m:
        return m


@pytest.mark.usefixtures("_patch_uuid4", "graph")
def test_run_workflow_default_run_id(graph: GraphData) -> None:
    inputs = {}
    run_workflow(inputs=inputs, workflow=graph)
    with Path.open(
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
        workflow=graph,
        run_id=31415,
        use_uv_executor=True,
        registry_path=Path(),
    )
