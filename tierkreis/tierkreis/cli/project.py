import argparse
import os
from pathlib import Path
import subprocess
import shutil


from tierkreis.cli.templates import (
    external_worker_idl,
    default_graph,
    python_worker_main,
    python_worker_pyproject,
)
from tierkreis.exceptions import TierkreisError
from tierkreis.namespace import Namespace


def parse_args(
    parser: argparse.ArgumentParser,
) -> argparse.ArgumentParser:
    init_subparsers = parser.add_subparsers(
        dest="init_type",
        help="Initialize tierkreis related structures",
        required=True,
    )
    project = init_subparsers.add_parser(
        "project",
        description="Initialize and manages project wide options."
        " Please make sure to set up a python project first, e.g. by executing `uv init`.",
        help="Initializes a new tierkreis project and manages project wide options.",
    )

    project.add_argument(
        "--default_checkpoint_directory",
        help="Overwrites the default checkpoint directory and sets the environment variable TKR_DIR for the current shell."
        "If you want to persist this behavior add it to your systems environment. e.g. export TKR_DIR=... ",
        type=Path,
        default=Path.home() / ".tierkreis/checkpoints",
    )
    project.add_argument(
        "--worker_directory",
        help="Overwrites the default worker directory.",
        type=Path,
        default=Path(".") / "workers",
    )
    project.add_argument(
        "--graphs_directory",
        help="Overwrites the default graph directory.",
        type=Path,
        default=Path(".") / "graphs",
    )
    worker = init_subparsers.add_parser(
        "worker",
        help="Generates a new worker.",
    )
    worker.add_argument(
        "--worker_directory",
        help="Overwrites the default worker directory.",
        type=str,
        default=Path(".") / "workers",
    )
    worker.add_argument(
        "--external",
        help="Set this flag for non-python workers. This will generate an IDL file instead of python related files.",
        action="store_true",
    )
    worker.add_argument(
        "-n", "--name", required=True, help="The name of the new worker", type=str
    )
    init_subparsers.add_parser("stubs", help="Generates worker stubs with UV.")
    return parser


def _gen_worker(worker_name: str, worker_dir: Path, external: bool = False) -> None:
    base_dir = worker_dir / worker_name
    base_dir.mkdir(exist_ok=True)
    with open(base_dir / "README.md", "w+", encoding="utf-8") as fh:
        fh.write(f"# {worker_name} \n")
    if external:
        with open(base_dir / f"{worker_name}.tsp", "w+", encoding="utf-8") as fh:
            fh.write(external_worker_idl(worker_name))
        return
    with open(base_dir / "main.py", "w+", encoding="utf-8") as fh:
        fh.write(python_worker_main(worker_name))
    with open(base_dir / "pyproject.toml", "w+", encoding="utf-8") as fh:
        fh.write(python_worker_pyproject(worker_name))


def _gen_stubs(worker_directory: Path) -> None:
    uv_path = shutil.which("uv")
    if uv_path is None:
        raise TierkreisError("uv is required to use the uv_executor")
    for worker in worker_directory.iterdir():
        if not worker.is_dir():
            continue
        if (idl := next(worker.glob("*.tsp"), None)) is not None:
            namespace = Namespace.from_spec_file(idl)
            namespace.write_stubs(idl.parent / "stubs.py")
        else:
            subprocess.run(
                [uv_path, "run", "main.py", "--stubs-path", "./stubs.py"], cwd=worker
            )


def run_args(args: argparse.Namespace) -> None:
    if args.init_type == "project":
        worker_name = "example_worker"
        args.worker_directory.mkdir(exist_ok=True, parents=True)
        _gen_worker(worker_name, args.worker_directory)
        args.graphs_directory.mkdir(exist_ok=True, parents=True)
        with open(args.graphs_directory / "main.py", "w+", encoding="utf-8") as fh:
            fh.write(default_graph(worker_name))
        os.environ["TKR_DIR"] = str(args.default_checkpoint_directory)
        _gen_stubs(args.worker_directory)
    elif args.init_type == "worker":
        args.worker_directory.mkdir(exist_ok=True, parents=True)
        _gen_worker(args.name, args.external)
    elif args.init_type == "stubs":
        print("Stubs")


class TierkreisInitCli:
    @staticmethod
    def add_subcommand(
        main_parser: argparse._SubParsersAction,
    ) -> None:
        parser = main_parser.add_parser(
            "init",
            description="Initializes the tierkreis project resources",
            help="Initializes the tierkreis project resources. Run `tkr init --help` for more information.",
        )
        parser = parse_args(parser)
        parser.set_defaults(func=TierkreisInitCli.execute)

    @staticmethod
    def execute(args: argparse.Namespace) -> None:
        run_args(args)
