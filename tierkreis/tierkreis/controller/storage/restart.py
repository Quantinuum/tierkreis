from typing import assert_never
from tierkreis.controller.data.location import Loc
from tierkreis.controller.storage.protocol import ControllerStorage


def dependents(storage: ControllerStorage, loc: Loc) -> set[Loc]:
    """Nodes that are fully invalidated if the node at the given loc is invalidated.

    This does not include the direct parent Loc, which is only partially invalidated."""
    descs: set[Loc] = set()

    parent = loc.parent()
    if parent is None:
        return set()

    step = loc.peek()
    match step:
        case "-":
            pass
        case ("N", _):
            nodedef = storage.read_node_def(loc)
            if nodedef.type == "output":
                descs.update(dependents(storage, parent))
            for output in nodedef.outputs.values():
                descs.add(parent.N(output))
                descs.update(dependents(storage, parent.N(output)))
        case ("M", _):
            descs.update(dependents(storage, parent))
        case ("L", idx):
            _, loop_node = loc.pop_last()
            latest_idx = storage.latest_loop_iteration(loop_node).peek_index()
            [descs.add(loop_node.L(i)) for i in range(idx + 1, latest_idx + 1)]
            descs.update(dependents(storage, loop_node))
        case _:
            assert_never(step)

    return descs


def restart_task(storage: ControllerStorage, loc: Loc) -> None:
    """Restart the node at the given loc.

    Fully dependent nodes will be removed from the storage.
    The parent locs will be partially invalidated."""

    # Remove fully invalidated nodes.
    deps = dependents(storage, loc)
    [storage.delete(storage.workflow_dir / a) for a in deps]

    # Mark partially invalidated nodes as not started and remove their outputs.
    partials = loc.partial_paths()
    [storage.delete(storage._nodedef_path(x)) for x in partials]
    [storage.delete(storage._done_path(x)) for x in partials]
    [storage.delete(storage.workflow_dir / a / "outputs") for a in partials]
