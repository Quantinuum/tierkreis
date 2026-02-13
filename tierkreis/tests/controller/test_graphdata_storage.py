from uuid import UUID
import pytest
from tests.controller.sample_graphdata import simple_eval, simple_map
from tierkreis.controller.data.core import PortID
from tierkreis.controller.data.graph import (
    Const,
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
    ],
)
def test_read_node_def(node_location_str: str, graph: GraphData, target: str) -> None:
    loc = Loc(node_location_str)
    storage = GraphDataStorage(UUID(int=0), graph)
    node_def = storage.read_node_def(loc)
    assert node_def == target


@pytest.mark.parametrize(
    ["node_location_str", "graph", "inputs", "outputs"],
    [
        (
            "-.N4.M0",
            simple_map(),
            {"doubler_input", "intercept"},
            {"doubler_output"},
        ),
    ],
)
def test_read_graph_def(
    node_location_str: str, graph: GraphData, inputs: set[PortID], outputs: set[PortID]
) -> None:
    loc = Loc(node_location_str)
    storage = GraphDataStorage(UUID(int=0), graph)
    graph_def = storage.read_graph_def(loc)
    assert graph_def.graph_inputs == inputs
    assert set(graph_def.output_ports) == outputs


@pytest.mark.parametrize(
    ["node_location_str", "graph", "port", "target"],
    [
        ("-.N0", simple_eval(), "value", b"0"),
        ("-.N1", simple_eval(), "value", b"6"),
        ("-.N4.M0", simple_map(), "doubler_output", b"null"),
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
        ("-.N4.M0", simple_map(), ["doubler_output"]),
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
        ("-.N4.M0", simple_map(), None),
        ("-.N4.M0.N1", simple_map(), Input("intercept", outputs={"intercept": 4})),
    ],
)
def test_graph_node_from_loc_node(
    node_location_str: str, graph: GraphData, target: str
) -> None:
    loc = Loc(node_location_str)
    node_def, _ = graph_node_from_loc(loc, graph)
    assert node_def == target


@pytest.mark.parametrize(
    ["node_location_str", "graph", "inputs", "outputs"],
    [
        (
            "-",
            simple_eval(),
            set(),
            {"simple_eval_output"},
        ),
        ("-.N3", simple_eval(), set(), {"simple_eval_output"}),
        ("-.N4.M0", simple_map(), {"doubler_input", "intercept"}, {"doubler_output"}),
    ],
)
def test_graph_node_from_loc_graph(
    node_location_str: str, graph: GraphData, inputs: set[PortID], outputs: set[PortID]
) -> None:
    loc = Loc(node_location_str)
    _, graph = graph_node_from_loc(loc, graph)
    assert graph.graph_inputs == inputs
    assert set(graph.output_ports) == outputs
