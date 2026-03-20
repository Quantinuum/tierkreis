"""Graph and node definitions.

(Computational) graphs are the underlying data structure for workflows in tierkreis.
A Graph is comprised on nodes (atomic operations) and edges (their values).
Nodes have named inputs referencing a previously computed value in the graph;
and named outputs referencing an id to look for the respective value.
Inputs and outputs are called ports.
The graph is constructed by mapping inputs off a node (by name) to the
outputs of a previous node.
"""

import logging
from collections.abc import Callable
from dataclasses import dataclass, field
from typing import Any, Literal, assert_never

from pydantic import BaseModel, RootModel

from tierkreis.controller.data.core import NodeIndex, PortID, ValueRef
from tierkreis.controller.data.location import Loc, OutputLoc
from tierkreis.controller.data.types import PType, ptype_from_bytes
from tierkreis.exceptions import TierkreisError

logger = logging.getLogger(__name__)


@dataclass
class NodeMetaData:
    has_breakpoint: bool = False


@dataclass
class NodeDefBase:
    """Map each out-port to the list of nodes that use it."""

    outputs: dict[PortID, list[NodeIndex]] = field(default_factory=dict, kw_only=True)
    annotations: NodeMetaData = field(default_factory=NodeMetaData, kw_only=True)


@dataclass
class Func(NodeDefBase):
    """A function node.

    Defines a task which is run by a worker on an executor.

    :fields:
        function_name (str): The function to run.
        inputs (dict[PortID, ValueRef]): The mapping of inputs to their values.
        outputs (dict[PortID, NodeIndex]): Typically not used on functions,
            for the sake of simplifying NodeData.
    """

    function_name: str
    inputs: dict[PortID, ValueRef]
    type: Literal["function"] = field(default="function")


@dataclass
class Eval(NodeDefBase):
    """An eval node.

    Evaluates a nested graph.
    Necessary for higher order operations.

    :fields:
        graph (ValueRef): The reference to a nested graph body.
        inputs (dict[PortID, ValueRef]): The mapping of inputs to their values.
        outputs (dict[PortID, NodeIndex]): Mapping from outer output names to respective
            output nodes (by index) in the nested graph.
    """

    graph: ValueRef
    inputs: dict[PortID, ValueRef]
    type: Literal["eval"] = field(default="eval")


@dataclass
class Loop(NodeDefBase):
    """A loop node.

    Evaluates a nested graph iteratively.
    Inputs are updated from the previous iteration.
    Loops until continue_port value evaluates to false.

    :fields:
        body (ValueRef): The reference to a nested graph body.
        inputs (dict[PortID, ValueRef]): The mapping of inputs to their values.
        continue_port: PortID: A named boolean port as stopping criterion.
        outputs (dict[PortID, NodeIndex]): Mapping from outer output names to respective
            output nodes (by index) in the nested graph.
        name (str | None): Used as debug data for loop tracing.
    """

    body: ValueRef
    inputs: dict[PortID, ValueRef]
    continue_port: PortID  # The port that specifies if the loop should continue.
    type: Literal["loop"] = field(default="loop")
    name: str | None = None


@dataclass
class Map(NodeDefBase):
    """A map node.

    Evaluates a nested graph concurrently for a set of values on one port.
    Maps have a * input which indicates the value to map over.
    Typically this is done by fold map a b c unfold [...] where a b c are the arbitrary
    but fixed inputs of the map.

    :fields:
        body (ValueRef): The reference to a nested graph body.
        inputs (dict[PortID, ValueRef]): The mapping of inputs to their values.
        outputs (dict[PortID, NodeIndex]): Typically not used on functions,
            for the sake of simplifying NodeData.
    """

    body: ValueRef
    inputs: dict[PortID, ValueRef]
    type: Literal["map"] = field(default="map")


@dataclass
class Const(NodeDefBase):
    """A constant node.

    :fields:
        value (Any): The constant value
        inputs (dict[PortID, ValueRef]): The mapping of inputs to their values.
            Typically "value" or "body"
        outputs (dict[PortID, NodeIndex]): Mapping from outer output names to respective
            output nodes (by index) in the nested graphs.
    """

    value: Any
    inputs: dict[PortID, ValueRef] = field(default_factory=dict)
    type: Literal["const"] = field(default="const")


@dataclass
class IfElse(NodeDefBase):
    """A lazy if else node.

    :fields:
        pred (ValueRef): Ref to a boolean value dictating which branch to evaluate.
        if_true (ValueRef): Branch to evaluate when pred is true.
        if_false (ValueRef): Branch to evaluate when pred is false.
        inputs (dict[PortID, ValueRef]): The mapping of inputs to their values.
            Typically pred and values for the branches.
        outputs (dict[PortID, NodeIndex]): Mapping from outer output names to respective
            output nodes (by index) in the branches.
    """

    pred: ValueRef
    if_true: ValueRef
    if_false: ValueRef
    inputs: dict[PortID, ValueRef] = field(default_factory=dict)
    type: Literal["ifelse"] = field(default="ifelse")


@dataclass
class EagerIfElse(NodeDefBase):
    """An eager if else node.

    :fields:
        pred (ValueRef): Ref to a boolean value dictating which value to forward.
        if_true (ValueRef): Branch to forward when pred is true.
        if_false (ValueRef): Branch to forward when pred is false.
        inputs (dict[PortID, ValueRef]): The mapping of inputs to their values.
            Typically pred and values for the branches.
        outputs (dict[PortID, NodeIndex]): Mapping from outer output names to respective
            output nodes (by index) in the branches.
    """

    pred: ValueRef
    if_true: ValueRef
    if_false: ValueRef
    inputs: dict[PortID, ValueRef] = field(default_factory=dict)

    type: Literal["eifelse"] = field(default="eifelse")


@dataclass
class Input(NodeDefBase):
    """An input node.

    :fields:
        name (str): The name of the input value.
        inputs (dict[PortID, ValueRef]): The mapping of inputs to their values,
            typically a single element.
        outputs (dict[PortID, NodeIndex]): Typically not used on inputs,
            for the sake of simplifying NodeData.
    """

    name: str
    inputs: dict[PortID, ValueRef] = field(default_factory=dict)
    type: Literal["input"] = field(default="input")


@dataclass
class Output(NodeDefBase):
    """An output node.

    :fields:
        inputs (dict[PortID, ValueRef]): The mapping of inputs to their values,
            typically a single element (e.g. computation -> output)
        outputs (dict[PortID, NodeIndex]): Typically only forwards itself.
    """

    inputs: dict[PortID, ValueRef]
    type: Literal["output"] = field(default="output")


NodeDef = Func | Eval | Loop | Map | Const | IfElse | EagerIfElse | Input | Output
NodeDefModel = RootModel[NodeDef]


def reindex_inputs(node: NodeDef, reindex: Callable[[ValueRef], ValueRef]) -> None:
    """Updates all the nodes inputs with the provided function.
    Mutates the node in place, but - note - leaves outputs unchanged."""
    node.inputs = {k: reindex(v) for k, v in node.inputs.items()}
    match node.type:
        case "eval":
            node.graph = reindex(node.graph)
        case "loop" | "map":
            node.body = reindex(node.body)
        case "ifelse" | "eifelse":
            node.pred = reindex(node.pred)
            node.if_true = reindex(node.if_true)
            node.if_false = reindex(node.if_false)
        case "const" | "function" | "input" | "output":
            pass
        case _:
            assert_never(node)


def in_edges(node: NodeDef) -> dict[PortID, ValueRef]:
    """Find the incoming edges of a node.

    Finds all the defined inputs and adds the special constructions:
    - Graph body for map, loop, eval
    - Prediction for ifelse
    - All nodes for eager if else

    :param node: The node to evaluate.
    :type node: NodeDef
    :return: Mapping of port names to value references.
    :rtype: dict[PortID, ValueRef]
    """
    parents = dict(node.inputs.items())

    match node.type:
        case "eval":
            parents["body"] = node.graph
        case "loop":
            parents["body"] = node.body
        case "map":
            parents["body"] = node.body
        case "ifelse" | "eifelse":
            parents["pred"] = node.pred
            parents["if_true"] = node.if_true
            parents["if_false"] = node.if_false
        case "const" | "function" | "input" | "output":
            pass
        case _:
            assert_never(node)

    return parents


class GraphData(BaseModel):
    """The model of a computational graph.

    Encapsulates the entire computation.
    Nodes are stored in a list, where the NodeIndex points to a unique node.
    Graphs have a single output which can be a Struct of multiple fields.

    :fields:
        nodes (list[NodeDef]): The list of nodes in a graph.
        fixed_inputs (dict[PortID, OutputLoc]): A dict of fixed inputs for the graph.
            They have values defined at construction time.
        graph_inputs: (set[PortID]): A set of user defined inputs at runtime.
        graph_output_idx (NodeIndex | None): The index of the output node.
            Graphs must have exactly one output to run.
        named_nodes (dict[str, NodeIndex]): Mapping of node names to their index in the
            list. This is used for debug information.

    """

    nodes: list[NodeDef] = []
    fixed_inputs: dict[PortID, OutputLoc] = {}
    graph_inputs: set[PortID] = set()
    graph_output_idx: NodeIndex | None = None
    named_nodes: dict[str, NodeIndex] = {}

    def input(self, name: str, metadata: NodeMetaData = NodeMetaData()) -> ValueRef:
        """Add an input name.

        :param name: The name of the input.
        :type name: str
        :return: The reference to that value.
        :rtype: ValueRef
        """
        return self.add(Input(name, annotations=metadata))(name)

    def const(self, value: PType, metadata: NodeMetaData = NodeMetaData()) -> ValueRef:
        """Add a constant value.

        :param value: The value to add.
        :type value: PType
        :return: The reference to that value.
        :rtype: ValueRef
        """
        return self.add(Const(value, annotations=metadata))("value")

    def func(
        self,
        function_name: str,
        inputs: dict[PortID, ValueRef],
        metadata: NodeMetaData = NodeMetaData(),
    ) -> Callable[[PortID], ValueRef]:
        """Add a function node (task).

        :param function_name: The name of the function.
        :type function_name: str
        :param inputs: The mapping of the input values.
        :type inputs: dict[PortID, ValueRef]
        :return: A function returning index given an output.
        :rtype: Callable[[PortID], ValueRef]
        """
        return self.add(Func(function_name, inputs, annotations=metadata))

    def eval(
        self,
        graph: ValueRef,
        inputs: dict[PortID, ValueRef],
        metadata: NodeMetaData = NodeMetaData(),
    ) -> Callable[[PortID], ValueRef]:
        """Add an eval node.

        :param graph: The nested graph to evaluate.
        :type graph: ValueRef
        :param inputs: The mapping of the input values.
        :type inputs: dict[PortID, ValueRef]
        :return: A function returning index given an output.
        :rtype: Callable[[PortID], ValueRef]
        """
        return self.add(Eval(graph, inputs, annotations=metadata))

    def loop(
        self,
        body: ValueRef,
        inputs: dict[PortID, ValueRef],
        continue_port: PortID,
        name: str | None = None,
        metadata: NodeMetaData = NodeMetaData(),
    ) -> Callable[[PortID], ValueRef]:
        """Add a loop node.

        :param body: The graph to loop over.
        :type body: ValueRef
        :param inputs: The mapping of the input values.
        :type inputs: dict[PortID, ValueRef]
        :param continue_port: The termination criterion port.
        :type continue_port: PortID
        :param name: Name of the loop for tracing, defaults to None
        :type name: str | None, optional
        :return: A function returning index given an output.
        :rtype: Callable[[PortID], ValueRef]
        """
        return self.add(
            Loop(body, inputs, continue_port, name=name, annotations=metadata)
        )

    def map(
        self,
        body: ValueRef,
        inputs: dict[PortID, ValueRef],
        metadata: NodeMetaData = NodeMetaData(),
    ) -> Callable[[PortID], ValueRef]:
        """Add a map node.

        :param body: The graph to map over.
        :type body: ValueRef
        :param inputs: The mapping of the input values.
        :type inputs: dict[PortID, ValueRef]
        :return:  A function returning index given an output.
        :rtype: Callable[[PortID], ValueRef]
        """
        return self.add(Map(body, inputs, annotations=metadata))

    def if_else(
        self,
        pred: ValueRef,
        if_true: ValueRef,
        if_false: ValueRef,
        metadata: NodeMetaData = NodeMetaData(),
    ) -> Callable[[PortID], ValueRef]:
        """Add an lazy if else node.

        :param pred: The reference to conditional value.
        :type pred: ValueRef
        :param if_true: The graph/value for the true branch.
        :type if_true: ValueRef
        :param if_false: The graph/value for the false branch.
        :type if_false: ValueRef
        :return:  A function returning index given an output.
        :rtype: Callable[[PortID], ValueRef]
        """
        return self.add(IfElse(pred, if_true, if_false, annotations=metadata))

    def eager_if_else(
        self,
        pred: ValueRef,
        if_true: ValueRef,
        if_false: ValueRef,
        metadata: NodeMetaData = NodeMetaData(),
    ) -> Callable[[PortID], ValueRef]:
        """Add an eager if else node.

        :param pred: The reference to conditional value.
        :type pred: ValueRef
        :param if_true: The graph/value for the true branch.
        :type if_true: ValueRef
        :param if_false: The graph/value for the false branch.
        :type if_false: ValueRef
        :return:  A function returning index given an output.
        :rtype: Callable[[PortID], ValueRef]
        """
        return self.add(EagerIfElse(pred, if_true, if_false, annotations=metadata))

    def output(
        self, inputs: dict[PortID, ValueRef], metadata: NodeMetaData = NodeMetaData()
    ) -> None:
        """Add an output node.

        Computation -> output.

        :param inputs: The inputs of the outup node.
        :type inputs: dict[PortID, ValueRef]
        """

        _ = self.add(Output(inputs, annotations=metadata))

    def add(self, node: NodeDef) -> Callable[[PortID], ValueRef]:
        """Add a node to the graph.

        :param node: The node to add.
        :type node: NodeDef
        :raises TierkreisError: If multiple outputs are added.
        :return: A function given the output name of a node returns
            the index of the node it corresponds to.
        :rtype: Callable[[PortID], ValueRef]
        """
        idx = len(self.nodes)
        self.nodes.append(node)
        match node.type:
            case "output":
                if self.graph_output_idx is not None:
                    msg = f"Graph already has output at index {self.graph_output_idx}"
                    raise TierkreisError(
                        msg,
                    )

                self.graph_output_idx = idx
            case "input":
                self.graph_inputs.add(node.name)
            case "const" | "eval" | "function" | "map" | "ifelse" | "eifelse":
                pass
            case "loop":
                if node.name is not None:
                    self.named_nodes[node.name] = idx
            case _:
                assert_never(node)

        for i, port in in_edges(node).values():
            self.nodes[i].outputs.setdefault(port, []).append(idx)

        return lambda k: (idx, k)

    def output_idx(self) -> NodeIndex:
        """Find the index of the graph output node.

        :raises TierkreisError: If the graph has no output.
        :raises TierkreisError: It the node at the index is not an output.
        :return: The index for the output node in self.nodes
        :rtype: NodeIndex
        """
        idx = self.graph_output_idx
        if idx is None:
            msg = "Graph has no output index."
            raise TierkreisError(msg)

        node = self.nodes[idx]
        if node.type != "output":
            msg = f"Expected output node at {idx} found {node}"
            raise TierkreisError(msg)

        return idx

    def remaining_inputs(self, provided_inputs: set[PortID]) -> set[PortID]:
        """Find the inputs for which no values are provided.

        :param provided_inputs: The list of already provided inputs.
        :type provided_inputs: set[PortID]
        :raises TierkreisError: If provided inputs would overwrite fixed inputs.
        :return: A set of input names which don't have an associated value.
        :rtype: set[PortID]
        """
        fixed_inputs = set(self.fixed_inputs.keys())
        if fixed_inputs & provided_inputs:
            msg = (
                f"Fixed inputs {fixed_inputs}"
                f" should not intersect provided inputs {provided_inputs}."
            )
            raise TierkreisError(
                msg,
            )

        actual_inputs = fixed_inputs.union(provided_inputs)
        return self.graph_inputs - actual_inputs


def graph_node_from_loc(
    node_location: Loc,
    graph: GraphData,
) -> tuple[NodeDef, GraphData]:
    """Find the node definition and graph of a nested graph given a loc.

    Nested graphs nodes are not indexed in their parent as their are
    represented by a single node. E.g. g_1.eval(const(g_2)) will only produce a single
    index although g_2 can contain many nodes.
    Locs on the other hand contain this information e.g -.N0.L0.N-1 is a virtual eval
    node.
    This functions recursively steps trough nested graph definitions like this to find
    a graph according to a flat loc.
    Assumes the first part of a loc can be found in current graph.

    :param node_location: The loc to search for.
    :type node_location: Loc
    :param graph: The current graph to search in.
    :type graph: GraphData
    :raises TierkreisError: On an empty graph of a malformed Loc
    :return: The node containing a graph and the graph itself.
    :rtype: tuple[NodeDef, GraphData]
    """
    if len(graph.nodes) == 0:
        msg = "Cannot convert location to node. Reason: Empty Graph"
        raise TierkreisError(msg)
    if node_location == "-":
        return Eval((-1, "body"), {}), graph

    step, remaining_location = node_location.pop_first()
    if isinstance(step, str):
        msg = "Cannot convert location: Reason: Malformed Loc"
        raise TierkreisError(msg)
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
            if len(remaining_location.steps()) <= 1:
                return Eval((-1, "body"), node.inputs, outputs=node.outputs), graph

            node, graph = graph_node_from_loc(remaining_location, graph)
        case "const" | "function" | "input" | "output" | "ifelse" | "eifelse":
            pass
        case _:
            assert_never(node)

    return node, graph


def _unwrap_graph(node: NodeDef, node_type: str) -> GraphData:
    """Safely unwraps a const nodes GraphData."""
    if not isinstance(node, Const):
        msg = (
            f"Cannot convert location to node. Reason: {node_type} does not wrap const"
        )
        raise TierkreisError(
            msg,
        )
    match node.value:
        case GraphData() as graph:
            return graph
        case bytes() as thunk:
            return GraphData(**ptype_from_bytes(thunk, dict))
        case dict() as data:
            return GraphData(**data)

        case _:
            msg = "Cannot convert location to node. Reason: const value is not a graph"
            raise TierkreisError(
                msg,
            )
