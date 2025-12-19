from pathlib import Path
import pytest
from tests.controller.sample_graphdata import simple_eval
from tests.controller.typed_graphdata import tkr_conj
from tierkreis.controller import debug_graph
from tierkreis.controller.data.location import Loc
from tierkreis.exceptions import TierkreisError
from tierkreis.storage import read_outputs
from tierkreis.builtins.stubs import iadd, itimes


def test_without_debug_values():
    g = simple_eval()
    registry_path = Path("./tierkreis/tierkreis")
    storage = debug_graph(
        g, graph_inputs={}, registry_path=registry_path, debug_values=None
    )

    actual_output = read_outputs(g, storage)
    assert actual_output == {"simple_eval_output": None}


def test_with_debug_values():
    g = simple_eval()
    debug_values = {iadd: 5}  # Literal is not a PModel
    registry_path = Path("./tierkreis/tierkreis")
    storage = debug_graph(
        g, graph_inputs={}, registry_path=registry_path, debug_values=debug_values
    )

    actual_output = read_outputs(g, storage)
    assert actual_output == {"simple_eval_output": 5}


def test_worker_defined_debug_values():
    g = tkr_conj()
    registry_path = Path("./tierkreis/tierkreis")
    storage = debug_graph(
        g,
        graph_inputs={"value": 2 + 2j},
        registry_path=registry_path,
        debug_values=None,
    )

    actual_output = read_outputs(g, storage)
    assert actual_output == 1 + 1j


def test_debug_raises():
    g = simple_eval()
    debug_values = {itimes: "nan"}
    registry_path = Path("./tierkreis/tierkreis")
    with pytest.raises(TierkreisError):
        storage = debug_graph(
            g, graph_inputs={}, registry_path=registry_path, debug_values=debug_values
        )
        assert storage.node_has_error(Loc("-.N3.N4"))  # iadd


def test_debug_callable():
    g = simple_eval()
    debug_values = {iadd: lambda x, y: x + y + 1, itimes: _my_itimes}
    registry_path = Path("./tierkreis/tierkreis")
    storage = debug_graph(
        g, graph_inputs={}, registry_path=registry_path, debug_values=debug_values
    )

    actual_output = read_outputs(g, storage)
    assert actual_output == {"simple_eval_output": 8}


def _my_itimes(a: int, b: int) -> int:
    return 7
