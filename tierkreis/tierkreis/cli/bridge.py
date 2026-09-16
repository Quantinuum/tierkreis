"""Private Python adapters used by the Rust ``tkr`` executable.

This module is intentionally not a public CLI. It isolates operations that require
Python reflection while the command-line orchestration lives in Rust.
"""

from __future__ import annotations

import argparse
import sys
from pathlib import Path

from tierkreis.cli.run import load_graph
from tierkreis.namespace import Namespace


def _parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(prog="python -m tierkreis.cli.bridge")
    commands = parser.add_subparsers(dest="command", required=True)

    export = commands.add_parser("export-graph")
    export.add_argument("specifier")

    stubs = commands.add_parser("stubs-from-idl")
    stubs.add_argument("idl", type=Path)
    stubs.add_argument("output", type=Path)
    return parser


def main() -> None:
    """Execute one compatibility adapter operation."""
    args = _parser().parse_args()
    if args.command == "export-graph":
        graph = load_graph(args.specifier)
        sys.stdout.write(graph.model_dump_json())
    elif args.command == "stubs-from-idl":
        Namespace.from_spec_file(args.idl).write_stubs(args.output)


if __name__ == "__main__":
    main()
