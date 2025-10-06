from typing import assert_never
from tierkreis.controller.data.location import Loc
from tierkreis.controller.storage.base import TKRStorage


def dependents(storage: TKRStorage, loc: Loc) -> set[Loc]:
    """Nodes that are fully invalidated if the node at the given loc is invalidated."""
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


def restart(storage: TKRStorage, loc: Loc) -> None:
    deps = dependents(storage, loc)
    partials = loc.partial_paths()

    subpaths = [storage.list_subpaths(storage.workflow_dir / x) for x in deps]
    [[storage.delete(a) for a in x] for x in subpaths]

    subpaths = [
        storage.list_subpaths(storage.workflow_dir / x / "outputs") for x in partials
    ]
    print(subpaths)
    [[storage.delete(a) for a in x] for x in subpaths]
    [storage.delete(storage._nodedef_path(x)) for x in partials]
    [storage.delete(storage._done_path(x)) for x in partials]
