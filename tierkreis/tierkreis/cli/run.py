"""Tierkreis CLI main entrypoint."""

from __future__ import annotations

import asyncio
import importlib
import json
import logging
import sys
from collections.abc import Callable
from pathlib import Path
from typing import TYPE_CHECKING, Any

from tierkreis._tierkreis import load_runtime_config, new_from_config
from tierkreis.builder import Graph
from tierkreis.controller.data.graph import GraphData
from tierkreis.controller.data.types import PType, Workflow, ptype_from_bytes
from tierkreis.exceptions import TierkreisError

if TYPE_CHECKING:
    import argparse
    import types

logger = logging.getLogger(__name__)


def _import_from_path(module_name: str, file_path: str) -> types.ModuleType:
    """Import a graph when supplied as a path to a python file."""
    spec = importlib.util.spec_from_file_location(module_name, file_path)  # type: ignore[no-untyped-call]
    module = importlib.util.module_from_spec(spec)  # type: ignore[no-untyped-call]
    sys.modules[module_name] = module
    spec.loader.exec_module(module)
    return module


def load_graph(graph_input: str) -> GraphData:
    """Load a graph from an argument string.

    Loads a graph similar to how python runs modules with "-m"

    :param graph_input: The argument string specifying the graph.
    :type graph_input: str
    :raises TierkreisError: If the argument string is invalid.
    :return: The loaded graph data.
    :rtype: GraphData
    """
    if ":" not in graph_input:
        msg = f"Invalid argument: {graph_input}"
        raise TierkreisError(msg)
    module_name, function_name = graph_input.split(":")
    if ".py" in module_name:
        module = _import_from_path("graph_module", module_name)
    else:
        module = importlib.import_module(module_name, __package__)

    graph_data = None
    graph_name: Any = getattr(module, function_name)
    if isinstance(graph_name, GraphData):
        graph_data = graph_name
    if isinstance(graph_name, Graph | Workflow):
        graph_data = graph_name.data
    if isinstance(graph_name, Callable):
        graph_object = graph_name()
        if isinstance(graph_object, GraphData):
            graph_data = graph_object
        if isinstance(graph_object, Graph | Workflow):
            graph_data = graph_object.data
    if graph_data is None:
        logger.error("Could not load object %s as GraphData", graph_data)
        raise ValueError("Could not load Graph")

    return graph_data


def _load_inputs(input_files: list[str]) -> dict[str, PType]:
    """Load the inputs to a graph."""
    if len(input_files) == 1 and input_files[0].endswith(".json"):
        with Path.open(Path(input_files[0])) as fh:
            return {k: json.dumps(v).encode() for k, v in json.load(fh).items()}
    inputs = {}
    if len(input_files) == 1 and input_files[0] == "None":
        return inputs
    for input_file in input_files:
        if ":" not in input_file:
            msg = f"Invalid argument: {input_file}"
            raise TierkreisError(msg)
        key, value = input_file.split(":")
        with Path.open(Path(value), "rb") as fh:
            inputs[key] = ptype_from_bytes(fh.read())
    return inputs


def parse_args(
    main_parser: argparse._SubParsersAction[argparse.ArgumentParser],
) -> argparse.ArgumentParser:
    """Parse the arguments for the 'run' subcommand.

    :param main_parser: The main parser to add the subcommand to.
    :type main_parser: argparse._SubParsersAction[argparse.ArgumentParser]
    :return: The parser for the 'run' subcommand.
    :rtype: argparse.ArgumentParser
    """
    parser = main_parser.add_parser(
        name="run",
        description="Tierkreis: a workflow engine for quantum HPC.",
    )
    graph = parser.add_mutually_exclusive_group()
    graph.add_argument(
        "-f",
        "--from-file",
        type=Path,
        dest="graph",
        help="Load a graph from a .json file",
    )
    graph.add_argument(
        "-g",
        "--graph-location",
        help="Fully qualifying name of a Callable () -> GraphData. "
        " Example: examples.hello_world.hello_world_graph:hello_graph"
        " or a path to a python file and function."
        " Example: examples/hello_world/hello_world_graph.py:hello_graph",
        type=str,
        dest="graph",
    )
    parser.set_defaults(graph="./tkr/graphs/main.py:workflow")
    parser.add_argument(
        "-i",
        "--input-files",
        nargs="*",
        help="Graph inputs:"
        " Either a single .json file or a key value list  port1:path1 port2:path2"
        " where path is a binary file.",
        default=["workflow_inputs.json"],
    )
    parser.add_argument("--name", default=None, type=str, help="Set a workflow name")
    parser.add_argument(
        "--config",
        type=Path,
        help="Path to the runtime configuration file."
        " Will use system defaults if not set.",
    )
    parser.add_argument(
        "-l",
        "--loglevel",
        default=logging.WARNING,
        choices=logging.getLevelNamesMapping().keys(),
        help="Set log level.",
    )
    parser.add_argument("-v", "--verbose", action="store_true")
    parser.add_argument(
        "-o",
        "--print-output",
        action="store_true",
        help="Print the outputs of the top-level node. ",
    )

    return parser


async def run_workflow_new(args: argparse.Namespace) -> None:
    """Run a Tierkreis workflow using the new method."""
    config = load_runtime_config(args.config)
    if args.verbose:
        config.set_log_level("debug")

    if ":" in str(args.graph):
        graph = load_graph(str(args.graph))
    else:
        with Path.open(args.graph) as fh:
            graph = ptype_from_bytes(fh.read().encode(), GraphData)
    inputs = _load_inputs(args.input_files) if args.input_files is not None else {}
    runtime = await new_from_config(config)
    with runtime:
        workflow_id = await runtime.save_workflow(args.name, graph)
        run_id = await runtime.start_new_run(workflow_id, inputs)
        await runtime.wait_for(run_id, timeout=30)
        if args.print_output:
            actual_output = await runtime.get_outputs(run_id)
            print(actual_output)


class TierkreisRunCli:
    """Tierkeirs cli for the `run` subcommand.

    Used to run graphs with tkr run ...
    """

    @staticmethod
    def add_subcommand(
        main_parser: argparse._SubParsersAction[argparse.ArgumentParser],
    ) -> None:
        """Add the run subcommand."""
        parser = parse_args(main_parser)
        parser.set_defaults(func=TierkreisRunCli.execute)

    @staticmethod
    def execute(args: argparse.Namespace) -> None:
        """Execute the run subcommand."""
        asyncio.run(run_workflow_new(args))
