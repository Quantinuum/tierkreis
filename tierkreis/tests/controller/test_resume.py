from pathlib import Path
from typing import Any
from uuid import UUID

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
    eval_body_is_from_worker,
    eval_graph_of_graph,
    eval_from_worker_with_graph_from_worker,
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
    embed_graph,
)
from tierkreis.controller import run_graph
from tierkreis.controller.data.graph import GraphData
from tierkreis.controller.data.location import Loc
from tierkreis.controller.data.types import PType
from tierkreis.controller.executor.in_memory_executor import InMemoryExecutor
from tierkreis.controller.executor.uv_executor import UvExecutor
from tierkreis.controller.storage.data import WorkflowMetaData
from tierkreis.controller.storage.filestorage import ControllerFileStorage
from tierkreis.controller.storage.in_memory import ControllerInMemoryStorage
from tierkreis.models import Workflow
from tierkreis.storage import read_outputs
from tierkreis._tierkreis import run_workflow

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
    (simple_ifelse(), 1, "simple_ifelse", {"pred": b"true"}),
    (simple_ifelse(), 2, "simple_ifelse", {"pred": b"false"}),
    (factorial(), 24, "factorial", 4),
    (loop_multiple_acc_untyped(), {"acc1": 6, "acc2": 12, "acc3": 18}, "multi_acc", {}),
    (loop_multiple_acc(), {"acc1": 6, "acc2": 12, "acc3": 18}, "multi_acc", {}),
    (simple_eagerifelse(), 1, "simple_eagerifelse", {"pred": b"true"}),
    (factorial(), 120, "factorial", {"value": b"5"}),
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
params: list[tuple[GraphData | Workflow, Any, str, int, dict[str, PType] | PType]] = [
    (graph, output, name, i + 1, inputs)
    for i, (graph, output, name, inputs) in enumerate(param_data)
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

storage_classes = [ControllerFileStorage, ControllerInMemoryStorage]
storage_ids = ["FileStorage", "In-memory"]


@pytest.mark.parametrize("storage_class", storage_classes, ids=storage_ids)
@pytest.mark.parametrize(
    ("graph", "output", "name", "workflow_id", "inputs"),
    params,
    ids=ids,
)
def test_resume(  # noqa: PLR0913
    storage_class: type[ControllerFileStorage | ControllerInMemoryStorage],
    graph: GraphData,
    output: dict[str, PType] | PType,
    name: str,
    workflow_id: int,
    inputs: dict[str, PType] | PType,
) -> None:
    g = graph
    storage = storage_class(UUID(int=workflow_id), name=name)
    test_workers_path = Path(__file__).parent.parent / "test_workers"
    executor = UvExecutor(test_workers_path, storage.logs_path)
    if isinstance(storage, ControllerInMemoryStorage):
        executor = InMemoryExecutor(Path("./tierkreis/tierkreis"), storage=storage)
    storage.clean_graph_files()
    run_graph(storage, executor, g, inputs)

    actual_output = read_outputs(g, storage)
    assert actual_output == output
    if not isinstance(storage, ControllerInMemoryStorage):
        wf_metadata = WorkflowMetaData(**storage.read_metadata(Loc()))
        assert wf_metadata.completion_time is not None
        assert wf_metadata.duration is not None and wf_metadata.duration > 0
        assert wf_metadata.name == name


@pytest.mark.parametrize(
    ("graph", "output", "name", "workflow_id", "inputs"),
    params,
    ids=ids,
)
def test_runtime(  # noqa: PLR0913
    graph: GraphData | Workflow,
    output: dict[str, PType] | PType,
    name: str,
    workflow_id: int,
    inputs: dict[str, PType] | PType,
) -> None:
    if isinstance(graph, Workflow):
        g = graph.data
    else:
        g = graph

    if "defaults" in name:
        pytest.skip("default arguments not supported")

    run_outputs = run_workflow(name, g, inputs)
    assert output == run_outputs


with_worker_param_data: list[
    tuple[GraphData | Workflow, Any, str, dict[str, PType] | PType]
] = [
    (eval_body_is_from_worker(), 21, "eval_body_is_from_worker", {"value": 10}),
    (eval_graph_of_graph(), 31, "eval_graph_of_graph", {"value": 3}),
    (
        eval_from_worker_with_graph_from_worker(),
        23,
        "eval_from_worker_with_graph_from_worker",
        {"value": 5},
    ),
]

with_worker_params: list[
    tuple[GraphData | Workflow, Any, str, int, dict[str, PType] | PType]
] = [
    (graph, output, name, i + 1, inputs)
    for i, (graph, output, name, inputs) in enumerate(with_worker_param_data)
]
with_worker_ids = [
    "eval_body_is_from_worker",
    "eval_graph_of_graph",
    "eval_from_worker_with_graph_from_worker",
]


@pytest.mark.parametrize(
    ("graph", "output", "name", "workflow_id", "inputs"),
    with_worker_params,
    ids=with_worker_ids,
)
def test_resume_with_worker(
    graph: GraphData,
    output: dict[str, PType] | PType,
    name: str,
    workflow_id: int,
    inputs: dict[str, PType] | PType,
) -> None:
    g = graph
    storage = ControllerFileStorage(UUID(int=workflow_id), name=name)
    test_workers_path = Path(__file__).parent.parent / "workers"
    executor = UvExecutor(test_workers_path, storage.logs_path)
    storage.clean_graph_files()
    run_graph(storage, executor, g, inputs)

    actual_output = read_outputs(g, storage)
    assert actual_output == output
