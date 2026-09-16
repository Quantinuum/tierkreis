"""Python packaging shim for the Rust ``tkr`` command-line interface."""

from __future__ import annotations

import sys

from tierkreis._tierkreis import run_cli  # type: ignore[import-not-found]


def main() -> None:
    """Forward the process arguments to the Rust CLI implementation."""
    run_cli(sys.argv[1:])


if __name__ == "__main__":
    main()
