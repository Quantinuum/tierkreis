import argparse
import logging
import sys

from tierkreis.cli.run import TierkreisRunCli
from tierkreis.cli.project import TierkreisInitCli


def main() -> None:
    parser = argparse.ArgumentParser(
        prog="tkr",
        description="Tierkreis: a workflow engine for quantum HPC. This is the main tierkreis command-line tool.",
    )
    subparser = parser.add_subparsers(title="subcommands")
    TierkreisRunCli.add_subcommand(subparser)
    TierkreisInitCli.add_subcommand(subparser)
    try:
        from tierkreis_visualization.cli import TierkreisVizCli

        TierkreisVizCli.add_subcommand(subparser)
    except ImportError:
        logging.warning("Could not import Tierkreis Visualization CLI")
        logging.warning(
            "To install it, please run 'pip install tierkreis-visualization'"
        )
    args = parser.parse_args(args=None if sys.argv[1:] else ["--help"])
    args.func(args)


if __name__ == "__main__":
    main()
