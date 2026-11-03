"""Tierkreis CLI main entrypoint."""

import argparse
import sys
from pathlib import Path

from tierkreis._tierkreis import run_builtin_worker_call


def main() -> None:
    """Run the main entry point for the builtins worker."""
    parser = argparse.ArgumentParser(
        prog="tkr-builtins", description="Builtin worker tasks for Tierkreis"
    )
    parser.add_argument("call_args", type=Path)
    parser.set_defaults(func=run_builtin_worker_call)
    args = parser.parse_args(args=None if sys.argv[1:] else ["--help"])
    run_builtin_worker_call(args.call_args)
