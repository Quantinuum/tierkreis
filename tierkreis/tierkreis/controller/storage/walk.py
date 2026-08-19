"""Functions to walk a (computational) workflow graph.

In conjunction with `start()` this is one of the primary functions of the
tierkreis controller.
By walking the graph we update nodes with new inputs from finished nodes until
they can be started.
"""

from logging import getLogger
from typing import assert_never

from tierkreis.controller.consts import BODY_PORT
from tierkreis.controller.data.core import NodeIndex
from tierkreis.controller.data.graph import (
    EagerIfElse,
    Eval,
    GraphData,
    Loop,
    Map,
    NodeDef,
)
from tierkreis.controller.data.location import Loc
from tierkreis.controller.data.types import ptype_from_bytes
from tierkreis.controller.storage.adjacency import outputs_iter, unfinished_inputs
from tierkreis.controller.storage.protocol import ControllerStorage
from tierkreis.controller.storage.walk_result import NodeRunData, WalkResult
from tierkreis.labels import Labels

logger = getLogger(__name__)


def unfinished_results(
    result: WalkResult,
    storage: ControllerStorage,
    parent: Loc,
    node: NodeDef,
    graph: GraphData,
) -> int:
    """Find and walk all the unfinished results.

    Finds all "blocking" nodes of the current nodes and marks them to be started.
    Blocking nodes are inputs that are not done yet.
    We walk recursively into the nodes that are not finished yet to progress them,
    by marking them ready for starting or done.

    :param result: The walk result, where we add all unfinished nodes.
        Used to bubble up the nodes from the recursive walk call.
    :type result: WalkResult
    :param storage: The storage to write to.
    :type storage: ControllerStorage
    :param parent: The parent node of the current node.
        e.g, the eval node containing the current node.
    :type parent: Loc
    :param node: The current node for which we check inputs.
    :type node: NodeDef
    :param graph: The graph to walk.
    :type graph: GraphData
    :return: The number of nodes that have unfinished inputs.
    :rtype: int
    """
    unfinished = unfinished_inputs(storage, parent, node)
    [result.extend(walk_node(storage, parent, x[0], graph)) for x in unfinished]
    return len(unfinished)


def walk_node(
    storage: ControllerStorage,
    parent: Loc,
    idx: NodeIndex,
    graph: GraphData,
) -> WalkResult:
    """Walk a graph node.

    Should only be called when a node has not finished.
    This is one of the core functions of the controller.
    It checks for a current node how to proceed:
        - Continue if its already done
        - Mark it for starting if its inputs are now ready and its not started.
        - Mark the respective next nodes to start, e.g. in case of an eval start
            the first nodes inside (that now have their inputs ready).

    :param storage: The storage to write to.
    :type storage: ControllerStorage
    :param parent: The parent node of the current node.
        e.g, the eval node containing the current node.
    :type parent: Loc
    :param idx: The index (respective to the parent) of the current node.
    :type idx: NodeIndex
    :param graph: The graph to walk.
    :type graph: GraphData
    :return: A summary list of finished, errored, and ready nodes.
    :rtype: WalkResult
    """
    loc = parent.N(idx)
    if storage.node_has_error(loc):
        # we immediately stop if a node has an error and bubble the error up
        logger.error("Node %s has encountered an error.", loc)
        logger.debug("\n\n%s\n\n", storage.read_errors(loc))
        return WalkResult(inputs_ready=[], started=[], errored=[loc])

    node = graph.nodes[idx]
    node_run_data = NodeRunData(
        node_location=loc, node=node, output_list=list(node.outputs)
    )

    result = WalkResult(inputs_ready=[], started=[])
    if unfinished_results(result, storage, parent, node, graph):
        # cannot start, don't have all inputs yet
        return result

    if not storage.is_node_started(loc):
        # have all inputs, start current node
        if idx in graph.node_metadata and graph.node_metadata[idx].has_breakpoint:
            return WalkResult(
                inputs_ready=[node_run_data], started=[], breakpoints=[loc]
            )
        return WalkResult(inputs_ready=[node_run_data], started=[])

    # Handle cases where we have nested graphs.
    # Basically we have to forward the now available outputs from outer scope
    # into the scope of the nested graph, so we check inside the nested
    # for new candidates.
    match node.type:
        case "eval":
            # step inside the nested graph for walking
            message = storage.read_output(parent.N(node.graph[0]), node.graph[1])
            g = ptype_from_bytes(message, GraphData)
            return walk_node(storage, loc, g.output_idx(), g)

        case "output":
            return WalkResult(inputs_ready=[node_run_data], started=[])

        case "const":
            return WalkResult(inputs_ready=[node_run_data], started=[])

        case "loop":
            return walk_loop(storage, parent, idx, node)

        case "map":
            return walk_map(storage, parent, idx, node)

        case "ifelse":
            # walk the next node only after we have the value of "pred"
            pred = storage.read_output(parent.N(node.pred[0]), node.pred[1])
            next_node = node.if_true if pred == b"true" else node.if_false
            next_loc = parent.N(next_node[0])
            if storage.is_node_finished(next_loc):
                storage.link_outputs(loc, Labels.VALUE, next_loc, next_node[1])
                storage.mark_node_finished(loc)
                return WalkResult(inputs_ready=[], started=[])
            return walk_node(storage, parent, next_node[0], graph)

        case "eifelse":
            return walk_eifelse(storage, parent, idx, node)

        case "function":
            # Current node can start, done will be marked by executor.
            return WalkResult(inputs_ready=[], started=[loc])

        case "input":
            return WalkResult(inputs_ready=[], started=[])
        case _:
            assert_never(node)


def walk_loop(
    storage: ControllerStorage,
    parent: Loc,
    idx: NodeIndex,
    loop: Loop,
) -> WalkResult:
    """Walk a loop node.

    The controller walks a loop by:
    - checking the current iteration
    - checking the `should_continue` port
    - mapping outputs to inputs between iterations
    - and finally producing the outputs.

    Each iteration is evaluated by inserting a virtual eval node
    at the location PARENT_LOC.L<ITER>.N-1 that then gets picked up by walk
    and the next start step.

    :param storage: The storage to write to.
    :type storage: ControllerStorage
    :param parent: The parent node of the current node.
        E.g. the "eval" containing this statement.
    :type parent: Loc
    :param idx: The index (respective to the parent) of the current node.
    :type idx: NodeIndex
    :param loop: The loop node being walked.
    :type loop: Loop
    :return: A summary list of finished, errored, and ready nodes:
        Either empty (loop is done), or next loop iteration.
    :rtype: WalkResult
    """
    loc = parent.N(idx)
    if storage.is_node_finished(loc):
        return WalkResult(inputs_ready=[], started=[])

    # find the last iteration
    new_location = storage.latest_loop_iteration(loc)
    # and read the graph definition
    message = storage.read_output(loc.N(-1), BODY_PORT)
    g = ptype_from_bytes(message, GraphData)
    loop_outputs = g.nodes[g.output_idx()].inputs
    # if the iteration is not done finished, walk its nested graph (body)
    if not storage.is_node_finished(new_location):
        return walk_node(storage, new_location, g.output_idx(), g)

    # Latest iteration is finished. Do we BREAK or CONTINUE?
    should_continue = ptype_from_bytes(
        storage.read_output(new_location, loop.continue_port),
        bool,
    )
    if should_continue is False:
        # were done here, set outputs to parent scope
        for k in loop_outputs:
            storage.link_outputs(loc, k, new_location, k)
        storage.mark_node_finished(loc)
        return WalkResult(inputs_ready=[], started=[])

    # continue looping, provide the inputs for the next iter from the current
    ins = {k: (-1, k) for k in loop.inputs}
    ins.update(loop_outputs)
    # Mark the next node as ready
    node_run_data = NodeRunData(
        node_location=loc.L(new_location.peek_index() + 1),
        node=Eval((-1, BODY_PORT), ins, outputs=loop.outputs),
        output_list=list(loop_outputs.keys()),
    )
    return WalkResult(inputs_ready=[node_run_data], started=[])


def walk_map(
    storage: ControllerStorage,
    parent: Loc,
    idx: NodeIndex,
    map_node: Map,
) -> WalkResult:
    """Walk a map node.

    We refer to the evaluation of the graph with a set of inputs as map elements.
    I. e. one map consist map fun [a,b,c] will have the element fun a, fun b, fun c.
    Each of these elements are treated as an virtual eval
    at the location PARENT_LOC.M<ele_index>.N-1 that then gets picked up by walk.
    In contrast to loop, as all map elements can be immediately started this is set
    in the start function, which makes the virtual evals optional;
    They are treated differently than evals/loops.

    :param storage: The storage to write to.
    :type storage: ControllerStorage
    :param parent: The parent node of the current node.
        E.g. the "eval" containing this statement.
    :type parent: Loc
    :param idx: The index (respective to the parent) of the current node.
    :type idx: NodeIndex
    :param map_node: The map node being walked.
    :type map_node: Map
    :return: A summary list of finished, errored, and ready nodes:
        Either empty (map is done), or all intermediate nodes in the map elements.
    :rtype: WalkResult
    """
    loc = parent.N(idx)
    result = WalkResult(inputs_ready=[], started=[])
    if storage.is_node_finished(loc):
        return result

    # find all values to map over
    first_ref = next(x for x in map_node.inputs.values() if x[1] == "*")
    map_eles = outputs_iter(storage, parent.N(first_ref[0]))
    # find all map elements that are not done
    unfinished = [i for i, _ in map_eles if not storage.is_node_finished(loc.M(i))]
    # Read the graph def
    message = storage.read_output(loc.M(0).N(-1), BODY_PORT)
    g = ptype_from_bytes(message, GraphData)
    # Walk all map elements simultaneously
    [result.extend(walk_node(storage, loc.M(p), g.output_idx(), g)) for p in unfinished]

    if len(unfinished) > 0:
        return result
    # All map elements are done, mark the entire map done
    map_outputs = g.nodes[g.output_idx()].inputs
    for i, j in map_eles:
        for output in map_outputs:
            storage.link_outputs(loc, f"{output}-{j}", loc.M(i), output)

    storage.mark_node_finished(loc)
    return result


def walk_eifelse(
    storage: ControllerStorage,
    parent: Loc,
    idx: NodeIndex,
    node: EagerIfElse,
) -> WalkResult:
    """Walk an eager if else node.

    In an eager if else node we have already evaluated all its
    inputs (pred, if_true, if_false).
    Therefore we just need to move the correct inputs to its outputs.

    :param storage: The storage to write to.
    :type storage: ControllerStorage
    :param parent: The parent node of the current node.
        E.g. the "eval" containing this statement.
    :type parent: Loc
    :param idx: The index (respective to the parent) of the current node.
    :type idx: NodeIndex
    :param node: The eager if else node being walked.
    :type node: EagerIfElse
    :return: An empty walk result since we here nothing else is to do.
    :rtype: WalkResult
    """
    loc = parent.N(idx)
    pred = storage.read_output(parent.N(node.pred[0]), node.pred[1])
    next_node = node.if_true if pred == b"true" else node.if_false
    next_loc = parent.N(next_node[0])
    storage.link_outputs(loc, Labels.VALUE, next_loc, next_node[1])
    storage.mark_node_finished(loc)

    return WalkResult(inputs_ready=[], started=[])
