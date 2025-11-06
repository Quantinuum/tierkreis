from dataclasses import dataclass, field
import logging
from typing import Any, Callable, Literal, assert_never
from pydantic import BaseModel, RootModel
from tierkreis.controller.data.core import PortID
from tierkreis.controller.data.core import NodeIndex
from tierkreis.controller.data.core import ValueRef
from tierkreis.controller.data.location import Loc, OutputLoc
from tierkreis.controller.data.types import PType, ptype_from_bytes
from tierkreis.exceptions import TierkreisError

logger = logging.getLogger(__name__)


@dataclass
class Func:
    function_name: str
    inputs: dict[PortID, ValueRef]
    outputs: dict[PortID, NodeIndex] = field(default_factory=lambda: {})
    type: Literal["function"] = field(default="function")


@dataclass
class Eval:
    graph: ValueRef
    inputs: dict[PortID, ValueRef]
    outputs: dict[PortID, NodeIndex] = field(default_factory=lambda: {})
    type: Literal["eval"] = field(default="eval")


@dataclass
class Loop:
    body: ValueRef
    inputs: dict[PortID, ValueRef]
    continue_port: PortID  # The port that specifies if the loop should continue.
    outputs: dict[PortID, NodeIndex] = field(default_factory=lambda: {})
    type: Literal["loop"] = field(default="loop")


@dataclass
class Map:
    body: ValueRef
    inputs: dict[PortID, ValueRef]
    outputs: dict[PortID, NodeIndex] = field(default_factory=lambda: {})
    type: Literal["map"] = field(default="map")


@dataclass
class Const:
    value: Any
    outputs: dict[PortID, NodeIndex] = field(default_factory=lambda: {})
    inputs: dict[PortID, ValueRef] = field(default_factory=lambda: {})
    type: Literal["const"] = field(default="const")


@dataclass
class IfElse:
    pred: ValueRef
    if_true: ValueRef
    if_false: ValueRef
    outputs: dict[PortID, NodeIndex] = field(default_factory=lambda: {})
    inputs: dict[PortID, ValueRef] = field(default_factory=lambda: {})
    type: Literal["ifelse"] = field(default="ifelse")


@dataclass
class EagerIfElse:
    pred: ValueRef
    if_true: ValueRef
    if_false: ValueRef
    outputs: dict[PortID, NodeIndex] = field(default_factory=lambda: {})
    inputs: dict[PortID, ValueRef] = field(default_factory=lambda: {})

    type: Literal["eifelse"] = field(default="eifelse")


@dataclass
class Input:
    name: str
    outputs: dict[PortID, NodeIndex] = field(default_factory=lambda: {})
    inputs: dict[PortID, ValueRef] = field(default_factory=lambda: {})
    type: Literal["input"] = field(default="input")


@dataclass
class Output:
    inputs: dict[PortID, ValueRef]
    outputs: dict[PortID, NodeIndex] = field(default_factory=lambda: {})
    type: Literal["output"] = field(default="output")


NodeDef = Func | Eval | Loop | Map | Const | IfElse | EagerIfElse | Input | Output
NodeDefModel = RootModel[NodeDef]


class GraphData(BaseModel):
    nodes: list[NodeDef] = []
    fixed_inputs: dict[PortID, OutputLoc] = {}
    graph_inputs: set[PortID] = set()
    graph_output_idx: NodeIndex | None = None

    def input(self, name: str) -> ValueRef:
        return self.add(Input(name))(name)

    def const(self, value: PType) -> ValueRef:
        return self.add(Const(value))("value")

    def func(
        self, function_name: str, inputs: dict[PortID, ValueRef]
    ) -> Callable[[PortID], ValueRef]:
        return self.add(Func(function_name, inputs))

    def eval(
        self, graph: ValueRef, inputs: dict[PortID, ValueRef]
    ) -> Callable[[PortID], ValueRef]:
        return self.add(Eval(graph, inputs))

    def loop(
        self, body: ValueRef, inputs: dict[PortID, ValueRef], continue_port: PortID
    ) -> Callable[[PortID], ValueRef]:
        return self.add(Loop(body, inputs, continue_port))

    def map(
        self,
        body: ValueRef,
        inputs: dict[PortID, ValueRef],
    ) -> Callable[[PortID], ValueRef]:
        return self.add(Map(body, inputs))

    def if_else(self, pred: ValueRef, if_true: ValueRef, if_false: ValueRef):
        return self.add(IfElse(pred, if_true, if_false))

    def eager_if_else(self, pred: ValueRef, if_true: ValueRef, if_false: ValueRef):
        return self.add(EagerIfElse(pred, if_true, if_false))

    def output(self, inputs: dict[PortID, ValueRef]) -> None:
        self.add(Output(inputs))

    def add(self, node: NodeDef) -> Callable[[PortID], ValueRef]:
        idx = len(self.nodes)
        self.nodes.append(node)

        match node.type:
            case "output":
                if self.graph_output_idx is not None:
                    raise TierkreisError(
                        f"Graph already has output at index {self.graph_output_idx}"
                    )

                self.graph_output_idx = idx
            case "ifelse" | "eifelse":
                self.nodes[node.pred[0]].outputs[node.pred[1]] = idx
                self.nodes[node.if_true[0]].outputs[node.if_true[1]] = idx
                self.nodes[node.if_false[0]].outputs[node.if_false[1]] = idx
            case "input":
                self.graph_inputs.add(node.name)
            case "const" | "eval" | "function" | "loop" | "map":
                pass
            case _:
                assert_never(node)

        for i, port in node.inputs.values():
            self.nodes[i].outputs[port] = idx

        return lambda k: (idx, k)

    def output_idx(self) -> NodeIndex:
        idx = self.graph_output_idx
        if idx is None:
            raise TierkreisError("Graph has no output index.")

        node = self.nodes[idx]
        if node.type != "output":
            raise TierkreisError(f"Expected output node at {idx} found {node}")

        return idx

    def remaining_inputs(self, provided_inputs: set[PortID]) -> set[PortID]:
        fixed_inputs = set(self.fixed_inputs.keys())
        if fixed_inputs & provided_inputs:
            raise TierkreisError(
                f"Fixed inputs {fixed_inputs}"
                f" should not intersect provided inputs {provided_inputs}."
            )

        actual_inputs = fixed_inputs.union(provided_inputs)
        return self.graph_inputs - actual_inputs


def graph_node_from_loc(
    node_location: Loc,
    graph: GraphData,
) -> tuple[NodeDef, GraphData]:
    """Assumes the first part of a loc can be found in current graph"""
    if len(graph.nodes) == 0:
        raise TierkreisError("Cannot convert location to node. Reason: Empty Graph")
    if node_location == "-":
        return Eval((-1, "body"), {}), graph

    step, remaining_location = node_location.pop_first()
    if isinstance(step, str):
        raise TierkreisError("Cannot convert location: Reason: Malformed Loc")
    (_, node_id) = step
    if node_id == -1:
        return Eval((-1, "body"), {}), graph
    node = graph.nodes[node_id]
    if remaining_location == Loc():
        return node, graph
    match node.type:
        case "eval":
            graph = _unwrap_graph(graph.nodes[node.graph[0]], node.type)
            node, graph = graph_node_from_loc(remaining_location, graph)
        case "loop" | "map":
            graph = _unwrap_graph(graph.nodes[node.body[0]], node.type)
            _, remaining_location = remaining_location.pop_first()  # Remove the M0/L0
            if len(remaining_location.steps()) < 2:
                return Eval((-1, "body"), node.inputs, node.outputs), graph

            node, graph = graph_node_from_loc(remaining_location, graph)
        case "const" | "function" | "input" | "output" | "ifelse" | "eifelse":
            pass
        case _:
            assert_never(node)

    return node, graph


def _unwrap_graph(node: NodeDef, node_type: str) -> GraphData:
    """Safely unwraps a const nodes GraphData."""
    if not isinstance(node, Const):
        raise TierkreisError(
            f"Cannot convert location to node. Reason: {node_type} does not wrap const"
        )
    match node.value:
        case GraphData() as graph:
            return graph
        case bytes() as thunk:
            return GraphData(**ptype_from_bytes(thunk, dict))
        case dict() as data:
            return GraphData(**data)

        case _:
            raise TierkreisError(
                "Cannot convert location to node. Reason: const value is not a graph"
            )
