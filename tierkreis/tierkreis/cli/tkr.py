"""Tierkreis CLI main entrypoint."""

from __future__ import annotations

import argparse
import logging
import sys

from tierkreis.cli.project import TierkreisInitCli
from tierkreis.cli.run import TierkreisRunCli

logger = logging.getLogger(__name__)


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
    try:
        from tierkreis_visualization.cli import TierkreisVizCli

        TierkreisVizCli.add_subcommand(subparser)
    except ImportError:
        logger.warning("Could not import Tierkreis Visualization CLI")
        logger.warning(
            "To install it, please run 'uv add tierkreis-visualization'",
        )
    args = parser.parse_args(args=None if sys.argv[1:] else ["--help"])
    args.func(args)


if __name__ == "__main__":
    main()
