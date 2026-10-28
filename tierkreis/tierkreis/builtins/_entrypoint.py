"""Console entry point for the Rust builtin worker."""

import json
import sys
from pathlib import Path

from tierkreis._tierkreis import (  # pyright: ignore[reportMissingImports]
    builtin_task_names,
    run_builtin_worker,
)


def main() -> None:
    """Run a builtin worker call or list the supported task names."""
    if sys.argv[1:] == ["--list"]:
        print(json.dumps(builtin_task_names()))
        return
    if len(sys.argv) != 2:
        msg = "usage: tkr-builtins [--list | CALL_ARGS]"
        raise SystemExit(msg)
    run_builtin_worker(Path(sys.argv[1]))
