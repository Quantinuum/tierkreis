from dataclasses import dataclass, field
from logging import getLogger
from typing import assert_never

from tierkreis.controller.data.core import NodeIndex, PortID
from tierkreis.controller.data.graph import (
    EagerIfElse,
    Eval,
    GraphData,
    Loop,
    Map,
)
from tierkreis.controller.data.location import Loc, OutputLoc
from tierkreis.controller.data.types import ptype_from_bytes
from tierkreis.controller.start import NodeRunData
from tierkreis.controller.storage.adjacency import outputs_iter, unfinished_inputs
from tierkreis.controller.storage.protocol import ControllerStorage
from tierkreis.labels import Labels

logger = getLogger(__name__)


@dataclass
class WalkResult:
    inputs_ready: list[NodeRunData]
    started: list[Loc]
    errored: list[Loc] = field(default_factory=list[Loc])

    def extend(self, walk_result: "WalkResult") -> None:
        self.inputs_ready.extend(walk_result.inputs_ready)
        self.started.extend(walk_result.started)
        self.errored.extend(walk_result.errored)


def walk_node(
    storage: ControllerStorage,
    parent: Loc,
    idx: NodeIndex,
    graph: GraphData,
    graph_inputs: dict[PortID, OutputLoc],
) -> WalkResult:
    """Should only be called when a node has not finished."""
    loc = parent.N(idx)
    if storage.node_has_error(loc):
        logger.error(f"Node {loc} has encountered an error:")
        logger.error(f"\n\n{storage.read_errors(loc)}\n\n")
        return WalkResult([], [], [loc])

    node = graph.nodes[idx]

    if node.type == "input":
        # Pass graph inputs for Input to pick whichever it wants
        node_run_data = NodeRunData(loc, node, [node.name], graph_inputs)
        return WalkResult([node_run_data], [])

    node_run_data = NodeRunData.from_node(loc, node, list(node.outputs))

    unfinished = unfinished_inputs(storage, parent, node)
    if len(unfinished) > 0:
        result = WalkResult([], [])
        for r in unfinished:
            result.extend(walk_node(storage, parent, r[0], graph, graph_inputs))
        return result

    if not storage.is_node_started(loc):
        return WalkResult([node_run_data], [])

    match node.type:
        case "eval":
            graph_loc: OutputLoc = (parent.N(node.graph[0]), node.graph[1])
            message = storage.read_output(*graph_loc)
            g = ptype_from_bytes(message, GraphData)
            graph_inputs = node_run_data.inputs  # node_run_data unused
            assert graph_inputs["body"] == graph_loc
            return walk_node(storage, loc, g.output_idx(), g, graph_inputs)

        case "output":
            return WalkResult([node_run_data], [])

        case "const":
            return WalkResult([node_run_data], [])

        case "loop":
            return walk_loop(storage, parent, idx, node)

        case "map":
            return walk_map(storage, parent, idx, node)

        case "ifelse":
            pred = storage.read_output(parent.N(node.pred[0]), node.pred[1])
            next_node = node.if_true if pred == b"true" else node.if_false
            next_loc = parent.N(next_node[0])
            if storage.is_node_finished(next_loc):
                storage.link_outputs(loc, Labels.VALUE, next_loc, next_node[1])
                storage.mark_node_finished(loc)
                return WalkResult([], [])
            else:
                return walk_node(storage, parent, next_node[0], graph, graph_inputs)

        case "eifelse":
            return walk_eifelse(storage, parent, idx, node)

        case "function":
            return WalkResult([], [loc])

        case _:  # includes "input"
            assert_never(node)


def walk_loop(
    storage: ControllerStorage, parent: Loc, idx: NodeIndex, loop: Loop
) -> WalkResult:
    loop_loc = parent.N(idx)
    if storage.is_node_finished(loop_loc):
        return WalkResult([], [], [])

    loop_input_vals = NodeRunData.from_node(loop_loc, loop, []).inputs
    graph_loc = loop_input_vals["body"]
    assert graph_loc == (parent.N(loop.body[0]), loop.body[1])
    message = storage.read_output(*graph_loc)
    g = ptype_from_bytes(message, GraphData)
    loop_output_ports = g.nodes[g.output_idx()].inputs

    current_iter = storage.latest_loop_iteration(loop_loc)
    current_iter_loc = loop_loc.L(current_iter)
    if storage.is_node_finished(current_iter_loc):
        # Latest iteration is finished. Do we BREAK or CONTINUE?
        should_continue = ptype_from_bytes(
            storage.read_output(current_iter_loc, loop.continue_port), bool
        )
        if should_continue is False:
            for k in loop_output_ports:
                storage.link_outputs(loop_loc, k, current_iter_loc, k)
            storage.mark_node_finished(loop_loc)
            return WalkResult([], [])
        # Begin new iter!
        current_iter += 1
        current_iter_loc = loop_loc.L(current_iter)
        start_new_iter = True
    else:
        start_new_iter = False

    # Compute iter inputs...first the inputs at the start of the loop (iter 0)
    if current_iter > 0:
        prev_iter_loc = loop_loc.L(current_iter - 1)
        # Override with outputs of previous iter, but keep original inputs
        # as they may be invariants
        loop_input_vals.update({k: (prev_iter_loc, k) for k in loop_output_ports})

    if start_new_iter:
        # We're doing this for the bit of `start` that applies regardless of the node
        # but that just records the node having started. (Do the inputs matter?
        # No - so those are just for visualisation/debugging - we could simplify
        # this function if we didn't record them)
        datum = NodeRunData(
            current_iter_loc,
            Eval((-1, "**dummy-never-read**"), loop.inputs, {}),
            [],
            loop_input_vals,
        )
        return WalkResult([datum], [])
    return walk_node(storage, current_iter_loc, g.output_idx(), g, loop_input_vals)


def walk_map(
    storage: ControllerStorage, parent: Loc, idx: NodeIndex, map: Map
) -> WalkResult:
    loc = parent.N(idx)
    result = WalkResult([], [])
    if storage.is_node_finished(loc):
        return result

    first_ref = next(x for x in map.inputs.values() if x[1] == "*")
    map_eles = outputs_iter(storage, parent.N(first_ref[0]))
    unfinished = [(i, e) for i, e in map_eles if not storage.is_node_finished(loc.M(i))]
    map_inputs = NodeRunData.from_node(loc, map, []).inputs
    graph_loc = map_inputs["body"]
    assert graph_loc == (parent.N(map.body[0]), map.body[1])
    message = storage.read_output(*graph_loc)
    g = ptype_from_bytes(message, GraphData)

    if len(unfinished) > 0:
        for idx, p in unfinished:
            eval_inputs = {"body": graph_loc}
            for k, (loc, port) in map_inputs.items():
                eval_inputs[k] = (loc, p if port == "*" else port)
            result.extend(
                walk_node(storage, loc.M(idx), g.output_idx(), g, eval_inputs)
            )

        return result

    map_outputs = g.nodes[g.output_idx()].inputs
    for i, j in map_eles:
        for output in map_outputs.keys():
            storage.link_outputs(loc, f"{output}-{j}", loc.M(i), output)

    storage.mark_node_finished(loc)
    return result


def walk_eifelse(
    storage: ControllerStorage,
    parent: Loc,
    idx: NodeIndex,
    node: EagerIfElse,
) -> WalkResult:
    loc = parent.N(idx)
    pred = storage.read_output(parent.N(node.pred[0]), node.pred[1])
    next_node = node.if_true if pred == b"true" else node.if_false
    next_loc = parent.N(next_node[0])
    storage.link_outputs(loc, Labels.VALUE, next_loc, next_node[1])
    storage.mark_node_finished(loc)

    return WalkResult([], [])
