"""Tierkreis CLI main entrypoint."""

from __future__ import annotations

import argparse
import sys

from tierkreis.cli.project import TierkreisInitCli
from tierkreis.cli.run import TierkreisRunCli
from tierkreis.viz import serve


def _serve(args: argparse.Namespace) -> None:
    serve(args.port)


def main() -> None:
    """Run the main entry point for the tkr cli."""
    parser = argparse.ArgumentParser(
        prog="tkr",
        description="Tierkreis: a workflow engine for quantum HPC."
        " This is the main tierkreis command-line tool.",
    )
    subparser = parser.add_subparsers(title="subcommands")
    TierkreisRunCli.add_subcommand(subparser)
    TierkreisInitCli.add_subcommand(subparser)
    serve_parser = subparser.add_parser("serve", help="Start the visualization server.")
    serve_parser.add_argument("--port", type=int, default=3000)
    serve_parser.set_defaults(func=_serve)
    args = parser.parse_args(args=None if sys.argv[1:] else ["--help"])
    args.func(args)


if __name__ == "__main__":
    main()
