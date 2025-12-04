from pathlib import Path
from typing import Any, Type
from uuid import UUID

import pytest

from tests.controller.sample_graphdata import (
    maps_in_series,
    simple_eagerifelse,
    simple_eval,
    simple_ifelse,
    simple_loop,
    simple_map,
)
from tests.controller.loop_graphdata import loop_multiple_acc, loop_multiple_acc_untyped
from tests.controller.typed_graphdata import (
    tkr_conj,
    tkr_list_conj,
    tuple_untuple,
    typed_destructuring,
    typed_eval,
    typed_loop,
    typed_map,
    factorial,
    gcd,
    typed_map_simple,
)
from tierkreis.controller import run_graph
from tierkreis.controller.data.types import PType
from tierkreis.controller.executor.in_memory_executor import InMemoryExecutor
from tierkreis.controller.executor.shell_executor import ShellExecutor
from tierkreis.controller.storage.filestorage import ControllerFileStorage
from tierkreis.controller.storage.in_memory import ControllerInMemoryStorage
from tierkreis.controller.data.graph import GraphData
from tierkreis.storage import read_outputs

params: list[tuple[GraphData, Any, str, int, dict[str, PType] | PType]] = [
    (simple_eval(), {"simple_eval_output": 12}, "simple_eval", 1, {}),
    (simple_loop(), 10, "simple_loop", 2, {}),
    (simple_map(), list(range(6, 47, 2)), "simple_map", 3, {}),
    (maps_in_series(), list(range(0, 81, 4)), "maps_in_series", 4, {}),
    (simple_ifelse(), 1, "simple_ifelse", 6, {"pred": b"true"}),
    (simple_ifelse(), 2, "simple_ifelse", 7, {"pred": b"false"}),
    (factorial().get_data(), 24, "factorial", 8, 4),
    (
        loop_multiple_acc_untyped(),
        {"acc1": 6, "acc2": 12, "acc3": 18},
        "multi_acc",
        9,
        {},
    ),
    (
        loop_multiple_acc().get_data(),
        {"acc1": 6, "acc2": 12, "acc3": 18},
        "multi_acc",
        9,
        {},
    ),
    (simple_eagerifelse(), 1, "simple_eagerifelse", 10, {"pred": b"true"}),
    (factorial().get_data(), 120, "factorial", 12, {"value": b"5"}),
    (typed_eval().get_data(), {"typed_eval_output": 12}, "typed_eval", 14, {}),
    (typed_loop().get_data(), 10, "typed_loop", 15, {}),
    (
        typed_map().get_data(),
        list(range(6, 47, 2)),
        "typed_map",
        16,
        {"value": list(range(21))},
    ),
    (typed_map().get_data(), [], "typed_map", 16, {"value": []}),
    (
        typed_map_simple().get_data(),
        list(range(0, 42, 2)),
        "typed_map",
        16,
        {"value": list(range(21))},
    ),
    (typed_map_simple().get_data(), [], "typed_map", 16, {"value": []}),
    (
        typed_destructuring().get_data(),
        list(range(6, 47, 2)),
        "typed_map",
        17,
        {"value": list(range(21))},
    ),
    (typed_destructuring().get_data(), [], "typed_map", 17, {"value": []}),
    (tuple_untuple().get_data(), 3, "tuple_untuple", 18, {}),
    (gcd().get_data(), 21, "gcd", 19, {"a": 1071, "b": 462}),
    (gcd().get_data(), 2, "gcd", 20, {"a": 12, "b": 26}),
    (gcd().get_data(), 24, "gcd", 21, {"a": 48, "b": 360}),
    (gcd().get_data(), 1, "gcd", 22, {"a": 9357, "b": 5864}),
    (gcd().get_data(), 3, "gcd", 23, {"a": 3, "b": 0}),
    (tkr_conj().get_data(), complex(1, -1), "tkr_conj", 24, complex(1, 1)),
    (
        tkr_list_conj().get_data(),
        [complex(1, -1), complex(1, 0)],
        "tkr_conj",
        25,
        [complex(1, 1), complex(1, 0)],
    ),
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
]

storage_classes = [ControllerFileStorage, ControllerInMemoryStorage]
storage_ids = ["FileStorage", "In-memory"]


@pytest.mark.parametrize("storage_class", storage_classes, ids=storage_ids)
@pytest.mark.parametrize("graph,output,name,id,inputs", params, ids=ids)
def test_resume(
    storage_class: Type[ControllerFileStorage | ControllerInMemoryStorage],
    graph: GraphData,
    output: Any,
    name: str,
    id: int,
    inputs: dict[str, PType] | PType,
):
    g = graph
    storage = storage_class(UUID(int=id), name=name)
    executor = ShellExecutor(Path("./python/examples/launchers"), Path(""))
    if isinstance(storage, ControllerInMemoryStorage):
        executor = InMemoryExecutor(Path("./tierkreis/tierkreis"), storage=storage)
    storage.clean_graph_files()
    run_graph(storage, executor, g, inputs)

    actual_output = read_outputs(g, storage)
    assert actual_output == output
