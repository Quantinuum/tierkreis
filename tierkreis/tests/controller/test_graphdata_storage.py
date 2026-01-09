from uuid import UUID
import pytest
from tests.controller.sample_graphdata import simple_eval, simple_map
from tierkreis.controller.data.location import Loc
from tierkreis.controller.storage.graphdata import GraphDataStorage
from tierkreis.exceptions import TierkreisError
from tierkreis_core import (
    PortID,
    GraphData,
    NodeDef,
    ExteriorRef,
    NodeDescription,
    ValueRef,
    NodeIndex,
)


@pytest.mark.parametrize(
    ["node_location_str", "graph", "target"],
    [
        ("-.N0", simple_eval(), NodeDef.Const(0)),
        ("-.N4.M0", simple_map(), NodeDef.Eval(ExteriorRef("body"), {})),
        ("-.N4.M0.E", simple_map(), NodeDef.Eval(ExteriorRef("body"), {})),
    ],
)
def test_read_nodedescription_definition(
    node_location_str: str, graph: GraphData, target: NodeDef
) -> None:
    loc = Loc(node_location_str)
    storage = GraphDataStorage(UUID(int=0), graph)
    node_def = storage.read_node_description(loc)
    assert node_def.definition == target


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
        (
            "-.N0",
            simple_eval(),
            NodeDescription(NodeDef.Const(0), outputs={"value": NodeIndex(3)}),
        ),
        (
            "-.N3.N1",
            simple_eval(),
            NodeDescription(
                NodeDef.Input("intercept"), outputs={"intercept": NodeIndex(4)}
            ),
        ),
        (
            "-.N3.N3",
            simple_eval(),
            NodeDescription(
                NodeDef.Func(
                    "builtins.itimes",
                    inputs={
                        "a": ValueRef(NodeIndex(0), "doubler_input"),
                        "b": ValueRef(NodeIndex(2), "value"),
                    },
                ),
                outputs={"value": NodeIndex(4)},
            ),
        ),
        (
            "-.E",
            simple_eval(),
            NodeDescription(NodeDef.Eval(ExteriorRef("body"), {})),
        ),
        (
            "-.N3.E",
            simple_eval(),
            NodeDescription(NodeDef.Eval(ExteriorRef("body"), {})),
        ),
        (
            "-.N4.M0",
            simple_map(),
            NodeDescription(
                NodeDef.Eval(
                    ExteriorRef("body"),
                    inputs={
                        "doubler_input": ValueRef(NodeIndex(2), "*"),
                        "intercept": ValueRef(NodeIndex(0), "value"),
                    },
                ),
                outputs={"*": NodeIndex(5)},
            ),
        ),
        (
            "-.N4.M0.E",
            simple_map(),
            NodeDescription(NodeDef.Eval(ExteriorRef("body"), {})),
        ),
        (
            "-.N4.M0.N1",
            simple_map(),
            NodeDescription(
                NodeDef.Input("intercept"), outputs={"intercept": NodeIndex(4)}
            ),
        ),
    ],
)
def test_graph_node_from_loc(
    node_location_str: str, graph: GraphData, target: NodeDescription
) -> None:
    loc = Loc(node_location_str)
    node_description = graph.query_node_description(loc)
    # TOOD: Avoiding comparing parent graphs
    assert node_description.definition == target.definition
    assert node_description.outputs == target.outputs
