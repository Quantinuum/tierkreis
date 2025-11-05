from dataclasses import dataclass
from logging import getLogger
import logging
from pathlib import Path
import subprocess
import sys

from tierkreis.controller.data.core import PortID
from tierkreis.controller.data.types import bytes_from_ptype, ptype_from_bytes
from tierkreis.controller.executor.in_memory_executor import InMemoryExecutor
from tierkreis.controller.storage.adjacency import outputs_iter
from typing_extensions import assert_never

from tierkreis.consts import PACKAGE_PATH
from tierkreis.controller.data.graph import Eval, GraphData, NodeDef
from tierkreis.controller.data.location import Loc, OutputLoc
from tierkreis.controller.executor.protocol import ControllerExecutor
from tierkreis.controller.storage.base import ControllerStorage
from tierkreis.controller.storage.in_memory import ControllerInMemoryStorage
from tierkreis.labels import Labels
from tierkreis.exceptions import TierkreisError

logger = logging.getLogger(__name__)


@dataclass
class NodeRunData:
    node_location: Loc
    node: NodeDef
    output_list: list[PortID]


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

    storage.write_node_def(node_location, node)

    parent = node_location.parent()
    if parent is None:
        raise TierkreisError(f"{node.type} node must have parent Loc.")

    ins = {k: (parent.N(idx), p) for k, (idx, p) in node.inputs.items()}

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
        input_loc = parent.N(-1)
        storage.link_outputs(node_location, node.name, input_loc, node.name)
        storage.mark_node_finished(node_location)

    elif node.type == "output":
        storage.mark_node_finished(node_location)

        pipe_inputs_to_output_location(storage, parent, ins)
        storage.mark_node_finished(parent)

    elif node.type == "const":
        bs = bytes_from_ptype(node.value)
        storage.write_output(node_location, Labels.VALUE, bs)
        storage.mark_node_finished(node_location)

    elif node.type == "eval":
        message = storage.read_output(parent.N(node.graph[0]), node.graph[1])
        g = ptype_from_bytes(message, GraphData)
        ins["body"] = (parent.N(node.graph[0]), node.graph[1])
        ins.update(g.fixed_inputs)

        pipe_inputs_to_output_location(storage, node_location.N(-1), ins)

    elif node.type == "loop":
        ins["body"] = (parent.N(node.body[0]), node.body[1])
        pipe_inputs_to_output_location(storage, node_location.N(-1), ins)
        start(
            storage,
            executor,
            NodeRunData(
                node_location.L(0),
                Eval((-1, "body"), {k: (-1, k) for k, _ in ins.items()}, node.outputs),
                output_list,
            ),
        )

    elif node.type == "map":
        first_ref = next(x for x in ins.values() if x[1] == "*")
        map_eles = outputs_iter(storage, first_ref[0])
        if not map_eles:
            storage.mark_node_finished(node_location)
        for idx, p in map_eles:
            eval_inputs: dict[PortID, tuple[Loc, PortID]] = {}
            eval_inputs["body"] = (parent.N(node.body[0]), node.body[1])
            for k, (i, port) in ins.items():
                if port == "*":
                    eval_inputs[k] = (i, p)
                else:
                    eval_inputs[k] = (i, port)
            pipe_inputs_to_output_location(
                storage, node_location.M(idx).N(-1), eval_inputs
            )
            # Necessary in the node visualization
            storage.write_node_def(
                node_location.M(idx), Eval((-1, "body"), node.inputs, node.outputs)
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
