from dataclasses import dataclass, field
from logging import getLogger
from typing import Mapping, assert_never

from tierkreis.controller.data.core import NodeIndex, PortID
from tierkreis.controller.data.graph import (
    EagerIfElse,
    GraphData,
    Loop,
    Map,
    NodeDef,
    ValueRef,
)
from tierkreis.controller.data.location import Loc
from tierkreis.controller.data.types import ptype_from_bytes
from tierkreis.controller.start import RunNodeTask, Task, LoopIterTask
from tierkreis.controller.storage.adjacency import outputs_iter, unfinished_inputs
from tierkreis.controller.storage.protocol import ControllerStorage
from tierkreis.labels import Labels

logger = getLogger(__name__)


@dataclass
class WalkResult:
    inputs_ready: list[Task]
    started: list[Loc]
    errored: list[Loc] = field(default_factory=list[Loc])

    def extend(self, walk_result: "WalkResult") -> None:
        self.inputs_ready.extend(walk_result.inputs_ready)
        self.started.extend(walk_result.started)
        self.errored.extend(walk_result.errored)


def unfinished_results(
    result: WalkResult,
    storage: ControllerStorage,
    parent: Loc,
    node: NodeDef,
    graph: GraphData,
) -> int:
    unfinished = unfinished_inputs(storage, parent, node)
    [result.extend(walk_node(storage, parent, x[0], graph)) for x in unfinished]
    return len(unfinished)


def walk_node(
    storage: ControllerStorage, parent: Loc, idx: NodeIndex, graph: GraphData
) -> WalkResult:
    """Should only be called when a node has not finished."""
    loc = parent.N(idx)
    if storage.node_has_error(loc):
        logger.error(f"Node {loc} has encountered an error.")
        logger.debug(f"\n\n{storage.read_errors(loc)}\n\n")
        return WalkResult([], [], [loc])

    node = graph.nodes[idx]
    run_node_task = RunNodeTask(loc, node, list(node.outputs))

    result = WalkResult([], [])
    if unfinished_results(result, storage, parent, node, graph):
        return result

    if not storage.is_node_started(loc):
        return WalkResult([run_node_task], [])

    match node.type:
        case "eval":
            message = storage.read_output(parent.N(node.graph[0]), node.graph[1])
            g = ptype_from_bytes(message, GraphData)
            return walk_node(storage, loc, g.output_idx(), g)

        case "output":
            return WalkResult([run_node_task], [])

        case "const":
            return WalkResult([run_node_task], [])

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
                return walk_node(storage, parent, next_node[0], graph)

        case "eifelse":
            return walk_eifelse(storage, parent, idx, node)

        case "function":
            return WalkResult([], [loc])

        case "input":
            return WalkResult([], [])
        case _:
            assert_never(node)


def walk_loop(
    storage: ControllerStorage, parent: Loc, idx: NodeIndex, loop: Loop
) -> WalkResult:
    loop_loc = parent.N(idx)
    if storage.is_node_finished(loop_loc):
        return WalkResult([], [])

    graph_input = (parent.N(loop.body[0]), loop.body[1])
    message = storage.read_output(*graph_input)
    g = ptype_from_bytes(message, GraphData)
    loop_outputs: Mapping[PortID, ValueRef] = g.nodes[g.output_idx()].inputs
    iter = storage.latest_loop_iteration(loop_loc)
    new_location = loop_loc.L(iter)

    if not storage.is_node_finished(new_location):
        return walk_node(storage, new_location, g.output_idx(), g)

    # Latest iteration is finished. Do we BREAK or CONTINUE?
    should_continue = ptype_from_bytes(
        storage.read_output(new_location, loop.continue_port), bool
    )
    if should_continue is False:
        for k in loop_outputs:
            storage.link_outputs(loop_loc, k, new_location, k)
        storage.mark_node_finished(loop_loc)
        return WalkResult([], [])

    # Grab loop inputs so we have any invariants
    ins = {k: (parent.N(n), p) for k, (n, p) in loop.inputs.items()}
    # Override with results from previous iter (copied here by Output node):
    ins.update({k: (new_location, k) for k in loop_outputs})
    task = LoopIterTask(loop_loc.L(iter + 1), graph_input, ins)

    # Mark the iter as started so latest_loop_iteration picks it up
    # (not really the right node def, but works for is_node_started):
    storage.write_graph_def(task.iter_location, graph_input)
    return WalkResult([task], [])


def walk_map(
    storage: ControllerStorage, parent: Loc, idx: NodeIndex, map: Map
) -> WalkResult:
    map_loc = parent.N(idx)
    result = WalkResult([], [])
    if storage.is_node_finished(map_loc):
        return result

    first_ref = next(x for x in map.inputs.values() if x[1] == "*")
    map_eles = outputs_iter(storage, parent.N(first_ref[0]))
    unfinished = [i for i, _ in map_eles if not storage.is_node_finished(map_loc.M(i))]
    graph_input = (parent.N(map.body[0]), map.body[1])
    message = storage.read_output(*graph_input)
    g = ptype_from_bytes(message, GraphData)

    if len(unfinished) > 0:
        for p in unfinished:
            result.extend(walk_node(storage, map_loc.M(p), g.output_idx(), g))
        return result

    map_outputs = g.nodes[g.output_idx()].inputs
    for i, j in map_eles:
        for output in map_outputs.keys():
            storage.link_outputs(map_loc, f"{output}-{j}", map_loc.M(i), output)

    storage.mark_node_finished(map_loc)
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
