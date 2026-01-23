from __future__ import annotations

import argparse
import importlib
import json
import logging
import sys
from pathlib import Path
from typing import Any, Callable

from tierkreis.cli.run_workflow import run_workflow
from tierkreis.controller.data.graph import GraphData
from tierkreis.controller.data.types import PType, ptype_from_bytes
from tierkreis.exceptions import TierkreisError


def _import_from_path(module_name: str, file_path: str) -> Any:
    spec = importlib.util.spec_from_file_location(module_name, file_path)  # type: ignore
    module = importlib.util.module_from_spec(spec)  # type: ignore
    sys.modules[module_name] = module
    spec.loader.exec_module(module)
    return module


def load_graph(graph_input: str) -> GraphData:
    if ":" not in graph_input:
        raise TierkreisError(f"Invalid argument: {graph_input}")
    module_name, function_name = graph_input.split(":")
    print(f"Loading graph from module '{module_name}' and function '{function_name}'")
    if ".py" in module_name:
        module = _import_from_path("graph_module", module_name)
    else:
        module = importlib.import_module(module_name, __package__)
    build_submission_graph: Callable[[], GraphData] = getattr(module, function_name)

    return build_submission_graph()


def _load_inputs(input_files: list[str]) -> dict[str, PType]:
    if len(input_files) == 1 and input_files[0].endswith(".json"):
        with open(input_files[0], "r") as fh:
            return {k: json.dumps(v).encode() for k, v in json.load(fh).items()}
    inputs = {}
    for input_file in input_files:
        if ":" not in input_file:
            raise TierkreisError(f"Invalid argument: {input_file}")
        key, value = input_file.split(":")
        with open(value, "rb") as fh:
            inputs[key] = ptype_from_bytes(fh.read())
    return inputs


def parse_args(
    main_parser: argparse._SubParsersAction[argparse.ArgumentParser],
) -> argparse.ArgumentParser:
    parser = main_parser.add_parser(
        name="run",
        description="Runs tierkreis graphs from the cli.",
        help="Runs tierkreis graphs. Run `tkr run --help` for more information.",
    )
    graph = parser.add_mutually_exclusive_group(required=True)
    graph.add_argument(
        "-f", "--from-file", type=Path, help="Load a graph from a .json file"
    )
    graph.add_argument(
        "-g",
        "--graph-location",
        help="Fully qualifying name of a Callable () -> GraphData. "
        + "Example: tierkreis.cli.sample_graph:simple_eval"
        + "Or a path to a python file and function."
        + "Example: examples/hello_world/hello_world_graph.py:hello_graph",
        type=str,
    )
    parser.add_argument(
        "-i",
        "--input-files",
        nargs="*",
        help="Graph inputs:"
        "Either a single .json file or a key value list  port1:path1 port2:path2"
        + "where path is a binary file.",
    )
    parser.add_argument(
        "--run-id", default=None, type=int, help="Set a workflow run id"
    )
    parser.add_argument("--name", default=None, type=str, help="Set a workflow name")
    parser.add_argument(
        "-l",
        "--loglevel",
        default=logging.WARNING,
        choices=logging.getLevelNamesMapping().keys(),
        help="Set log level.",
    )
    parser.add_argument("-v", "--verbose", action="store_true")
    parser.add_argument(
        "--registry-path", default=None, type=Path, help="Location of executable tasks."
    )
    parser.add_argument(
        "-o",
        "--print-output",
        action="store_true",
        help="Print the outputs of the top-level node. ",
    )

    # run_graph() arguments
    parser.add_argument(
        "-n",
        "--n-iterations",
        default=10**5,
        type=int,
        help="Set the maximum number of iterations.",
    )
    parser.add_argument(
        "-p",
        "--polling-interval-seconds",
        default=0.01,
        type=float,
        help="Set the controller tickrate.",
    )
    parser.add_argument(
        "-r",
        "--do-clean-restart",
        action="store_true",
        help="Clear graph files before running",
    )
    parser.add_argument("--uv", action="store_true", help="Use uv worker")

    return parser


def run_workflow_args(args: argparse.Namespace):
    if args.verbose:
        args.log_level = logging.DEBUG

    if args.graph_location is not None:
        graph = load_graph(args.graph_location)
    else:
        with open(args.from_file, "r") as fh:
            graph = ptype_from_bytes(fh.read().encode(), GraphData)
    if args.input_files is not None:
        inputs = _load_inputs(args.input_files)
    else:
        inputs = {}
    print(inputs)
    run_workflow(
        graph,
        inputs,
        name=args.name,
        run_id=args.run_id,
        registry_path=args.registry_path,
        use_uv_worker=args.uv,
        n_iterations=args.n_iterations,
        polling_interval_seconds=args.polling_interval_seconds,
        print_output=args.print_output,
    )


class TierkreisRunCli:
    @staticmethod
    def add_subcommand(
        main_parser: argparse._SubParsersAction[argparse.ArgumentParser],
    ) -> None:
        parser = parse_args(main_parser)
        parser.set_defaults(func=TierkreisRunCli.execute)

    @staticmethod
    def execute(args: argparse.Namespace) -> None:
        run_workflow_args(args)
