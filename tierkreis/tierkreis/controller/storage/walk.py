from dataclasses import dataclass, field
from logging import getLogger

from tierkreis.controller.consts import BODY_PORT
from tierkreis.controller.data.location import Loc
from tierkreis.controller.data.types import ptype_from_bytes
from tierkreis.controller.start import NodeRunData
from tierkreis.controller.storage.adjacency import outputs_iter, unfinished_inputs
from tierkreis.controller.storage.protocol import ControllerStorage
from tierkreis.labels import Labels
from tierkreis_core import (
    ExteriorRef,
    GraphData,
    NodeDef,
    NodeIndex,
    ValueRef,
    PortID,
    new_eval_root,
)

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


def unfinished_results(
    result: WalkResult,
    storage: ControllerStorage,
    parent: Loc,
    node: NodeDef,
    graph: GraphData,
) -> int:
    unfinished = unfinished_inputs(storage, parent, node)
    [result.extend(walk_node(storage, parent, x.node_index, graph)) for x in unfinished]
    return len(unfinished)


def walk_node(
    storage: ControllerStorage, parent: Loc, idx: NodeIndex, graph: GraphData
) -> WalkResult:
    """Should only be called when a node has not finished."""
    loc = parent.N(idx)
    if storage.node_has_error(loc):
        logger.error(f"Node {loc} has encountered an error:")
        logger.error(f"\n\n{storage.read_errors(loc)}\n\n")
        return WalkResult([], [], [loc])

    node = graph.get_nodedef(idx)
    graph_outputs = graph.outputs(idx)
    if graph_outputs is None:
        raise ValueError("Cannot walk a graph with no outputs.")

    node_run_data = NodeRunData(loc, node, graph_outputs)

    result = WalkResult([], [])
    if unfinished_results(result, storage, parent, node, graph):
        return result

    if not storage.is_node_started(loc):
        return WalkResult([node_run_data], [])

    match node:
        case NodeDef.Eval():
            message = storage.read_output(
                parent.extend_from_ref(node.body), node.body.port_id
            )
            g = ptype_from_bytes(message, GraphData)

            output_idx = g.output_idx()
            if output_idx is None:
                raise ValueError("Cannot walk a graph with no Output node.")

            return walk_node(storage, loc, output_idx, g)

        case NodeDef.Output():
            return WalkResult([node_run_data], [])

        case NodeDef.Const():
            return WalkResult([node_run_data], [])

        case NodeDef.Loop():
            return walk_loop(storage, parent, idx, node)

        case NodeDef.Map():
            return walk_map(storage, parent, idx, node)

        case NodeDef.IfElse():
            pred = storage.read_output(
                parent.extend_from_ref(node.pred), node.pred.port_id
            )
            next_node = node.if_true if pred == b"true" else node.if_false
            next_loc = parent.extend_from_ref(next_node)
            if storage.is_node_finished(next_loc):
                storage.link_outputs(loc, Labels.VALUE, next_loc, next_node.port_id)
                storage.mark_node_finished(loc)
                return WalkResult([], [])
            else:
                return walk_node(storage, parent, next_node.node_index, graph)

        case NodeDef.EagerIfElse():
            return walk_eagerifelse(storage, parent, idx, node)

        case NodeDef.Func():
            return WalkResult([], [loc])

        case NodeDef.Input():
            return WalkResult([], [])
        case _:
            raise ValueError(f"Unhandled NodeDef of type: {type(node)}")


def walk_loop(
    storage: ControllerStorage, parent: Loc, idx: NodeIndex, loop: NodeDef.Loop
) -> WalkResult:
    loc = parent.N(idx)
    if storage.is_node_finished(loc):
        return WalkResult([], [], [])

    new_location = storage.latest_loop_iteration(loc)

    message = storage.read_output(loc.exterior(), BODY_PORT)
    g = ptype_from_bytes(message, GraphData)
    output_idx = g.output_idx()
    if output_idx is None:
        raise ValueError("Cannot walk a graph with no Output node.")

    if not storage.is_node_finished(new_location):
        return walk_node(storage, new_location, output_idx, g)

    # The outputs from the previous iteration
    body_outputs = g.get_nodedef(output_idx).in_edges
    if body_outputs is None:
        raise ValueError("Loop body has no outputs.")

    # Latest iteration is finished. Do we BREAK or CONTINUE?
    should_continue = ptype_from_bytes(
        storage.read_output(new_location, loop.continue_port), bool
    )
    if should_continue is False:
        for k in body_outputs:
            storage.link_outputs(loc, k, new_location, k)
        storage.mark_node_finished(loc)
        return WalkResult([], [])

    # Create new exterior refs for the inputs to the loop node
    #
    # This allows us to re-use the inputs to the loop node
    # between iterations.
    ins: dict[PortID, ValueRef | ExteriorRef] = {
        k: ExteriorRef(k) for k in loop.inputs.keys()
    }
    # Update with the outputs of the subgraph.
    ins.update(body_outputs)

    previous_index = new_location.peek_index()
    if previous_index is None:
        # TODO: This should be impossible
        raise ValueError("Previous step is not a Loop step.")

    # The outputs from the previous iteration
    graph_outputs = g.graph_outputs()
    if graph_outputs is None:
        raise ValueError("Loop body has no outputs.")

    node_run_data = NodeRunData(
        loc.L(previous_index + 1),
        new_eval_root(ins),
        graph_outputs,
    )
    return WalkResult([node_run_data], [])


def walk_map(
    storage: ControllerStorage, parent: Loc, idx: NodeIndex, map: NodeDef.Map
) -> WalkResult:
    loc = parent.N(idx)
    result = WalkResult([], [])
    if storage.is_node_finished(loc):
        return result

    first_ref = next(x for x in map.inputs.values() if x.port_id == "*")
    map_eles = outputs_iter(storage, parent.extend_from_ref(first_ref))
    unfinished = [i for i, _ in map_eles if not storage.is_node_finished(loc.M(i))]
    message = storage.read_output(loc.M(0).exterior(), BODY_PORT)
    g = ptype_from_bytes(message, GraphData)
    output_idx = g.output_idx()
    if output_idx is None:
        raise ValueError("Cannot walk a graph with no Output node.")

    [result.extend(walk_node(storage, loc.M(p), output_idx, g)) for p in unfinished]

    if len(unfinished) > 0:
        return result

    map_outputs = g.get_nodedef(output_idx).in_edges
    for i, j in map_eles:
        for output in map_outputs.keys():
            storage.link_outputs(loc, f"{output}-{j}", loc.M(i), output)

    storage.mark_node_finished(loc)
    return result


def walk_eagerifelse(
    storage: ControllerStorage,
    parent: Loc,
    idx: NodeIndex,
    node: NodeDef.EagerIfElse,
) -> WalkResult:
    loc = parent.N(idx)
    pred = storage.read_output(parent.extend_from_ref(node.pred), node.pred.port_id)
    next_node = node.if_true if pred == b"true" else node.if_false
    next_loc = parent.extend_from_ref(next_node)
    storage.link_outputs(loc, Labels.VALUE, next_loc, next_node.port_id)
    storage.mark_node_finished(loc)

    return WalkResult([], [])
