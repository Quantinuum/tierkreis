"""Generate TSP and CWL Python APIs for the command-line worker examples."""

import argparse
from pathlib import Path

from tierkreis.namespace import Namespace

HERE = Path(__file__).resolve().parent


def generate_stubs(directory: Path) -> None:
    for example in ("openssl", "cat", "text"):
        definitions = HERE / example
        tsp = Namespace.from_spec_file(definitions / "schema.tsp")
        cwl = Namespace.from_cwl_files(
            sorted(definitions.glob("*.cwl")), name=f"{example}_worker"
        )
        tsp.write_stubs(directory / example / "tsp_api.py")
        cwl.write_stubs(directory / example / "cwl_api.py")


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--output-directory",
        type=Path,
        default=HERE / "generated",
        help="Directory for the generated Python APIs (default: generated/ beside this script).",
    )
    args = parser.parse_args()
    generate_stubs(args.output_directory)
    print(f"Generated TSP and CWL Python APIs in {args.output_directory.resolve()}")


if __name__ == "__main__":
    main()
