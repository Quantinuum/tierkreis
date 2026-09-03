import os
import sys
from collections.abc import Iterator
from pathlib import Path

import pytest

from tests.controller.defaults_graphs import (
    defaults_not_none,
    defaults_omit,
    defaults_passthru,
)
from tests.controller.loop_graphdata import (
    loop_multiple_acc,
    loop_multiple_acc_untyped,
    loop_scoping,
)
from tests.controller.sample_graphdata import (
    maps_in_series,
    simple_eagerifelse,
    simple_eval,
    simple_ifelse,
    simple_loop,
    simple_map,
)
from tests.controller.typed_graphdata import (
    embed_graph,
    eval_body_is_from_worker,
    eval_from_worker_with_graph_from_worker,
    eval_graph_of_graph,
    factorial,
    gcd,
    tkr_conj,
    tkr_list_conj,
    tuple_untuple,
    typed_destructuring,
    typed_eval,
    typed_loop,
    typed_map,
    typed_map_simple,
)
from tierkreis import Runtime, run_workflow
from tierkreis.controller.data.graph import GraphData
from tierkreis.controller.data.types import PType
from tierkreis.models import Workflow

WORKER_PATH = Path(__file__).parent.parent / "workers"

param_data: list[
    tuple[
        GraphData | Workflow,
        dict[str, PType] | PType,
        str,
        dict[str, PType] | PType,
    ]
] = [
    (simple_eval(), {"simple_eval_output": 12}, "simple_eval", {}),
    (simple_loop(), 10, "simple_loop", {}),
    (simple_map(), list(range(6, 47, 2)), "simple_map", {}),
    (maps_in_series(), list(range(0, 81, 4)), "maps_in_series", {}),
    (simple_ifelse(), 1, "simple_ifelse", {"pred": True}),
    (simple_ifelse(), 2, "simple_ifelse", {"pred": False}),
    (factorial(), 24, "factorial", 4),
    (loop_multiple_acc_untyped(), {"acc1": 6, "acc2": 12, "acc3": 18}, "multi_acc", {}),
    (loop_multiple_acc(), {"acc1": 6, "acc2": 12, "acc3": 18}, "multi_acc", {}),
    (simple_eagerifelse(), 1, "simple_eagerifelse", {"pred": True}),
    (factorial(), 120, "factorial", {"value": 5}),
    (typed_eval(), {"typed_eval_output": 12}, "typed_eval", {}),
    (typed_loop(), 10, "typed_loop", {}),
    (typed_map(), list(range(6, 47, 2)), "typed_map", {"value": list(range(21))}),
    (typed_map(), [], "typed_map", {"value": []}),
    (
        typed_map_simple(),
        list(range(0, 42, 2)),
        "typed_map",
        {"value": list(range(21))},
    ),
    (typed_map_simple(), [], "typed_map", {"value": []}),
    (
        typed_destructuring(),
        list(range(6, 47, 2)),
        "typed_destructuring",
        {"value": list(range(21))},
    ),
    (typed_destructuring(), [], "typed_destructuring", {"value": []}),
    (tuple_untuple(), 3, "tuple_untuple", {}),
    (gcd(), 21, "gcd", {"a": 1071, "b": 462}),
    (gcd(), 2, "gcd", {"a": 12, "b": 26}),
    (gcd(), 24, "gcd", {"a": 48, "b": 360}),
    (gcd(), 1, "gcd", {"a": 9357, "b": 5864}),
    (gcd(), 3, "gcd", {"a": 3, "b": 0}),
    (tkr_conj(), complex(1, -1), "tkr_conj", complex(1, 1)),
    (
        tkr_list_conj(),
        [complex(1, -1), complex(1, 0)],
        "tkr_conj",
        [complex(1, 1), complex(1, 0)],
    ),
    (loop_scoping(), {"result": 11}, "loop_scoping", {}),
    (
        defaults_omit(),
        {
            "range_1": list(range(1, 10)),
            "range_2": list(range(1, 10)),
            "range_3": list(range(1, 10, 2)),
        },
        "defaults_omit",
        {"start": 1, "stop": 10},
    ),
    (
        defaults_passthru(),
        {
            "range_1": list(range(1, 10)),
            "range_2": list(range(1, 10)),
            "range_3": list(range(1, 10, 2)),
            "extra_output": None,
        },
        "defaults_passthru",
        {"start": 1, "stop": 10},
    ),
    (
        defaults_not_none(),
        {
            "range_1": list(range(1, 10)),
            "range_2": list(range(1, 10)),
            "range_3": list(range(1, 10, 2)),
            "extra_output": 1,
        },
        "defaults_not_none",
        {"start": 1, "stop": 10},
    ),
    (embed_graph(), {"s1": "1", "s2": "4", "final": 2}, "embed_graph", 1),
]
ids = [
    "simple_eval",
    "simple_loop",
    "simple_map",
    "maps_in_series",
    "simple_ifelse_true",
    "simple_ifelse_false",
    "factorial_4",
    "loop_multiple_acc_untyped",
    "loop_multiple_acc",
    "simple_eagerifelse",
    "factorial_5",
    "typed_eval",
    "typed_loop",
    "typed_map",
    "typed_map_empty",
    "typed_map_simple",
    "typed_map_simple_empty",
    "typed_destructuring",
    "typed_destructuring_empty",
    "tuple_untuple",
    "gcd_1071_462",
    "gcd_12_26",
    "gcd_48_360",
    "gcd_9357_5864",
    "gcd_3_0",
    "tkr_conj",
    "tkr_conj_list",
    "loop_scoping",
    "defaults_omit",
    "defaults_passthru",
    "defaults_not_none",
    "embed_graph",
]


@pytest.mark.parametrize(
    ("graph", "output", "name", "inputs"),
    param_data,
    ids=ids,
)
def test_runtime(
    graph: GraphData | Workflow,
    output: dict[str, PType] | PType,
    name: str,
    inputs: dict[str, PType] | PType,
) -> None:
    if "defaults" in name:
        pytest.skip("omitted optional workflow inputs are not supported")

    with Runtime() as runtime:
        uploaded_workflow_id = runtime.upload_workflow(name, graph)
        run_id = runtime.start_workflow(uploaded_workflow_id, inputs)
        assert runtime.wait(run_id, timeout=30) == "Completed"
        assert output == runtime.get_outputs(run_id)
        state = runtime.get_workflow_state(run_id)
        assert state.workflow_id == uploaded_workflow_id
        assert state.run_id == run_id
        assert state.name == name
        assert state.status == "Completed"


def test_run_workflow_utility() -> None:
    """The one-shot helper retains its direct-output contract."""
    assert run_workflow("simple_eval", simple_eval(), {}) == {"simple_eval_output": 12}


with_worker_param_data = [
    (eval_body_is_from_worker(), 21, "eval_body_is_from_worker", {"value": 10}),
    (eval_graph_of_graph(), 31, "eval_graph_of_graph", {"value": 3}),
    pytest.param(
        eval_from_worker_with_graph_from_worker(),
        23,
        "eval_from_worker_with_graph_from_worker",
        {"value": 5},
        marks=pytest.mark.xfail(
            reason="reusing a workflow-valued input evaluates it only once",
            strict=True,
        ),
    ),
]

with_worker_ids = [
    "eval_body_is_from_worker",
    "eval_graph_of_graph",
    "eval_from_worker_with_graph_from_worker",
]


@pytest.fixture
def subprocess_runtime(
    tmp_path: Path,
    monkeypatch: pytest.MonkeyPatch,
) -> Iterator[Runtime]:
    """Rust runtime configured with the graph-producing Python worker."""
    executable = tmp_path / "tkr-graph"
    executable.write_text(
        f"#!{sys.executable}\n"
        "import runpy\n"
        f"runpy.run_path({str(WORKER_PATH / 'graph/main.py')!r}, run_name='__main__')\n"
    )
    executable.chmod(0o755)
    monkeypatch.setenv("PATH", f"{tmp_path}{os.pathsep}{os.environ['PATH']}")
    with Runtime.sqlite(
        tmp_path / "runtime.sqlite",
        tmp_path / "assets",
        executor="subprocess",
    ) as runtime:
        yield runtime


@pytest.mark.parametrize(
    ("graph", "output", "name", "inputs"),
    with_worker_param_data,
    ids=with_worker_ids,
)
def test_runtime_with_worker(
    subprocess_runtime: Runtime,
    graph: GraphData | Workflow,
    output: dict[str, PType] | PType,
    name: str,
    inputs: dict[str, PType] | PType,
) -> None:
    workflow_id = subprocess_runtime.upload_workflow(name, graph)
    run_id = subprocess_runtime.start_workflow(workflow_id, inputs)
    assert subprocess_runtime.wait(run_id, timeout=30) == "Completed"
    assert subprocess_runtime.get_outputs(run_id) == output
