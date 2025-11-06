from pathlib import Path
from uuid import UUID
import pytest
from tests.controller.sample_graphdata import simple_eval
from tierkreis import run_graph
from tierkreis.consts import PACKAGE_PATH
from tierkreis.controller.data.graph import GraphData
from tierkreis.controller.data.location import Loc
from tierkreis.controller.executor.uv_executor import UvExecutor
from tierkreis.storage import FileStorage
from tests.controller.typed_graphdata import typed_loop, typed_map

params = [
    (simple_eval(), Loc().N(0), {Loc().N(4), Loc().N(3)}),
    (
        simple_eval(),
        Loc().N(3).N(0),
        {Loc().N(3).N(3), Loc().N(3).N(4), Loc().N(3).N(5), Loc().N(4)},
    ),
    (
        typed_loop(),
        Loc().N(2).L(2).N(3),
        {Loc().N(2).L(2).N(4), Loc().N(2).L(2).N(5), Loc().N(2).L(3), Loc().N(3)},
    ),
    (
        typed_map(),
        Loc().N(4).M(5).N(3),
        {Loc().N(4).M(5).N(4), Loc().N(4).M(5).N(5), Loc().N(5), Loc().N(6)},
    ),
]


@pytest.mark.parametrize("graph,input_loc,expected", params)
def test_descendants(graph: GraphData, input_loc: Loc, expected: set[Loc]):
    storage = FileStorage(UUID(int=200), "test_descendants", do_cleanup=True)
    executor = UvExecutor(
        Path(f"{PACKAGE_PATH}/../tierkreis_workers"), storage.logs_path
    )
    run_graph(storage, executor, graph, {"value": list(range(21))})

    assert storage.exists(storage._nodedef_path(input_loc))
    assert storage.exists(storage._done_path(input_loc))

    for loc in expected:
        assert storage.exists(storage._nodedef_path(loc))
        assert storage.exists(storage._done_path(loc))

    assert expected == storage.dependents(input_loc)

    storage.restart_task(input_loc)
    assert not storage.exists(storage._nodedef_path(input_loc))
    assert not storage.exists(storage._done_path(input_loc))

    for loc in expected:
        assert not storage.exists(storage._nodedef_path(input_loc))
        assert not storage.exists(storage._done_path(loc))

    run_graph(storage, executor, graph, {})
