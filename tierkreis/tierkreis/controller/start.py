from dataclasses import dataclass
from logging import getLogger
import logging
from pathlib import Path
import subprocess
import sys

from tierkreis.controller.data.core import PortID
from tierkreis.controller.data.types import bytes_from_ptype
from tierkreis.controller.executor.in_memory_executor import InMemoryExecutor
from tierkreis.controller.storage.adjacency import outputs_iter, in_edges
from typing_extensions import assert_never

from tierkreis.consts import PACKAGE_PATH
from tierkreis.controller.data.graph import Eval, NodeDef
from tierkreis.controller.data.location import Loc, OutputLoc
from tierkreis.controller.executor.protocol import ControllerExecutor
from tierkreis.controller.storage.protocol import ControllerStorage
from tierkreis.controller.storage.in_memory import ControllerInMemoryStorage
from tierkreis.labels import Labels
from tierkreis.exceptions import TierkreisError

logger = logging.getLogger(__name__)


@dataclass
class NodeRunData:
    node_location: Loc
    node: NodeDef
    output_list: list[PortID]
    inputs: dict[PortID, OutputLoc]

    @property
    def parent_loc(self) -> Loc:
        parent = self.node_location.parent()
        assert parent is not None
        return parent

    @staticmethod
    def from_node(
        node_location: Loc, node: NodeDef, output_list: list[PortID]
    ) -> "NodeRunData":
        parent = node_location.parent()
        if parent is None:
            raise TierkreisError(f"{type(node)} node must have parent Loc.")

        inputs = {k: (parent.N(idx), p) for k, (idx, p) in in_edges(node).items()}

        return NodeRunData(
            node_location,
            node,
            output_list,
            inputs,
        )


def start_nodes(
    storage: ControllerStorage,
    executor: ControllerExecutor,
    node_run_data: list[NodeRunData],
) -> None:
    started_locs: set[Loc] = set()
    for node_run_datum in node_run_data:
        if node_run_datum.node_location in started_locs:
            continue
        start(storage, executor, node_run_datum)
        started_locs.add(node_run_datum.node_location)


def run_builtin(def_path: Path, logs_path: Path) -> None:
    logger = getLogger("builtins")
    if not logger.hasHandlers():
        formatter = logging.Formatter(
            fmt="%(asctime)s: %(message)s",
            datefmt="%Y-%m-%dT%H:%M:%S%z",
        )
        handler = logging.FileHandler(logs_path, mode="a")
        handler.setFormatter(formatter)
        logger.setLevel(logging.INFO)

        logger.addHandler(handler)

    logger.info("START builtin %s", def_path)
    with open(logs_path, "a") as fh:
        subprocess.Popen(
            [sys.executable, "main.py", def_path],
            start_new_session=True,
            cwd=PACKAGE_PATH / "tierkreis" / "builtins",
            stderr=fh,
            stdout=fh,
        )


def start(
    storage: ControllerStorage, executor: ControllerExecutor, node_run_data: NodeRunData
) -> None:
    node_location = node_run_data.node_location
    node = node_run_data.node
    output_list = node_run_data.output_list
    ins = node_run_data.inputs

    storage.write_node_def(node_location, node)

    logger.debug(f"start {node_location} {node} {ins} {output_list}")
    if node.type == "function":
        name = node.function_name
        launcher_name = ".".join(name.split(".")[:-1])
        name = name.split(".")[-1]
        call_args_path = storage.write_worker_call_args(
            node_location, name, ins, output_list
        )
        logger.debug(f"Executing {(str(node_location), name, ins, output_list)}")

        if isinstance(storage, ControllerInMemoryStorage) and isinstance(
            executor, InMemoryExecutor
        ):
            executor.run(launcher_name, call_args_path)
        elif launcher_name == "builtins":
            run_builtin(call_args_path, storage.logs_path)
        else:
            executor.run(launcher_name, call_args_path)

    elif node.type == "input":
        storage.link_outputs(node_location, node.name, *ins[node.name])
        storage.mark_node_finished(node_location)

    elif node.type == "output":
        storage.mark_node_finished(node_location)

        pipe_inputs_to_output_location(storage, node_run_data.parent_loc, ins)
        storage.mark_node_finished(node_run_data.parent_loc)

    elif node.type == "const":
        bs = bytes_from_ptype(node.value)
        storage.write_output(node_location, Labels.VALUE, bs)
        storage.mark_node_finished(node_location)

    elif node.type == "eval":
        # All parameter-gathering handled by walking, so nothing to do here
        # except handle fixed inputs... not sure what/how atm.

        # message = storage.read_output(parent.N(node.graph[0]), node.graph[1])
        # g = ptype_from_bytes(message, GraphData)
        # ins.update(g.fixed_inputs)
        pass
    elif node.type == "loop":
        if (
            node.name is not None
        ):  # should we do this only in debug mode? -> need to think through how this would work
            storage.write_debug_data(node.name, node_location)
    elif node.type == "map":
        first_ref = next(x for x in ins.values() if x[1] == "*")
        map_eles = outputs_iter(storage, first_ref[0])
        if not map_eles:
            storage.mark_node_finished(node_location)
        for idx, p in map_eles:
            # establishing inputs to each element is done by `walk_map`

            # Necessary in the node visualization
            storage.write_node_def(
                node_location.M(idx),
                Eval((-1, "**dummy-never-read**"), node.inputs, node.outputs),
            )

    elif node.type == "ifelse":
        pass

    elif node.type == "eifelse":
        pass
    else:
        assert_never(node)


def pipe_inputs_to_output_location(
    storage: ControllerStorage,
    output_loc: Loc,
    inputs: dict[PortID, OutputLoc],
) -> None:
    for new_port, (old_loc, old_port) in inputs.items():
        storage.link_outputs(output_loc, new_port, old_loc, old_port)
