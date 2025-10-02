from uuid import UUID
import pytest
from tests.controller.sample_graphdata import simple_eval, simple_map
from tierkreis.controller.data.core import PortID
from tierkreis.controller.data.graph import (
    Const,
    Eval,
    Func,
    GraphData,
    Input,
    graph_node_from_loc,
)
from tierkreis.controller.data.location import Loc
from tierkreis.controller.storage.graphdata import GraphDataStorage
from tierkreis.exceptions import TierkreisError


@pytest.mark.parametrize(
    ["node_location_str", "graph", "target"],
    [
        ("-.N0", simple_eval(), Const(0, outputs={"value": 3})),
        ("-.N4.M0", simple_map(), Eval((-1, "body"), {})),
        ("-.N4.M0.N-1", simple_map(), Eval((-1, "body"), {})),
    ],
)
def test_read_nodedef(node_location_str: str, graph: GraphData, target: str) -> None:
    loc = Loc(node_location_str)
    storage = GraphDataStorage(UUID(int=0), graph)
    node_def = storage.read_node_def(loc)
    assert node_def == target


@pytest.mark.parametrize(
    ["node_location_str", "graph", "port", "target"],
    [
        ("-.N0", simple_eval(), "value", b"null"),
        ("-.N4.M0", simple_map(), "0", b"null"),
    ],
)
def test_read_output(
    node_location_str: str, graph: GraphData, port: PortID, target: str
) -> None:
    loc = Loc(node_location_str)
    storage = GraphDataStorage(UUID(int=0), graph)
    val = storage.read_output(loc, port)
    assert val == target


def test_raises() -> None:
    loc = Loc("-.N0")
    storage = GraphDataStorage(UUID(int=0), simple_eval())
    with pytest.raises(TierkreisError):
        storage.read_output(loc, "does_not_exist")


@pytest.mark.parametrize(
    ["node_location_str", "graph", "target"],
    [
        ("-.N0", simple_eval(), ["value"]),
        ("-.N4.M0", simple_map(), ["0"]),
    ],
)
def test_read_output_ports(
    node_location_str: str, graph: GraphData, target: str
) -> None:
    loc = Loc(node_location_str)
    storage = GraphDataStorage(UUID(int=0), graph)
    outputs = storage.read_output_ports(loc)
    assert outputs == target


@pytest.mark.parametrize(
    ["node_location_str", "graph", "target"],
    [
        ("-.N0", simple_eval(), Const(0, outputs={"value": 3})),
        ("-.N3.N1", simple_eval(), Input("intercept", outputs={"intercept": 4})),
        (
            "-.N3.N3",
            simple_eval(),
            Func(
                "builtins.itimes",
                inputs={"a": (0, "doubler_input"), "b": (2, "value")},
                outputs={"value": 4},
            ),
        ),
        ("-.N-1", simple_eval(), Eval((-1, "body"), {})),
        ("-.N3.N-1", simple_eval(), Eval((-1, "body"), {})),
        (
            "-.N4.M0",
            simple_map(),
            Eval(
                (-1, "body"),
                inputs={"doubler_input": (2, "*"), "intercept": (0, "value")},
                outputs={"*": 5},
            ),
        ),
        ("-.N4.M0.N-1", simple_map(), Eval((-1, "body"), {})),
        ("-.N4.M0.N1", simple_map(), Input("intercept", outputs={"intercept": 4})),
    ],
)
def test_graph_node_from_loc(
    node_location_str: str, graph: GraphData, target: str
) -> None:
    loc = Loc(node_location_str)
    node_def, _ = graph_node_from_loc(loc, graph)
    assert node_def == target
