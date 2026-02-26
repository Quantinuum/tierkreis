"""CLI for project related operations."""

import argparse
import os
import shutil
import subprocess
from pathlib import Path

from tierkreis.cli.templates import (
    default_graph,
    external_worker_idl,
    python_worker_main,
    python_worker_pyproject,
    python_worker_workspace_pyproject,
)
from tierkreis.exceptions import TierkreisError
from tierkreis.namespace import Namespace


def parse_args(
    parser: argparse.ArgumentParser,
) -> argparse.ArgumentParser:
    """Parse the arguments for the init subcommand."""
    init_subparsers = parser.add_subparsers(
        dest="init_type",
        help="Initialize tierkreis related structures",
        required=True,
    )
    project = init_subparsers.add_parser(
        "project",
        description="Initialize and manages project wide options. "
        "Make sure to set up a python project first, e.g. by executing `uv init`.",
        help="Initializes a new tierkreis project and manages project wide options.",
    )

    project.add_argument(
        "--default-checkpoint-directory",
        help="""Overwrites the default checkpoint directory and sets the environment
        variable TKR_DIR for the current shell. If you want to persist this behavior
        add it to your systems environment. e.g. export TKR_DIR=... """,
        type=Path,
        default=Path.home() / ".tierkreis/checkpoints",
    )
    project.add_argument(
        "--project-directory",
        help="Sets the default project directory. ",
        type=Path,
        default=Path(),
    )
    project.add_argument(
        "--worker-directory",
        help="Overwrites the default worker directory."
        "Defaults to <project_directory>/workers.",
        type=Path,
        default=Path("./tkr") / "workers",
    )
    project.add_argument(
        "--graphs-directory",
        help="Overwrites the default graph directory.",
        type=Path,
        default=Path("./tkr") / "graphs",
    )
    worker = init_subparsers.add_parser(
        "worker",
        help="Generates a new worker.",
    )
    worker.add_argument(
        "--worker-directory",
        help="Overwrites the default worker directory."
        "Defaults to <project_directory>/workers.",
        type=str,
        default=Path("./tkr") / "workers",
    )
    worker.add_argument(
        "--external",
        help="Set this flag for non-python workers."
        " This will generate an IDL file instead of python related files.",
        action="store_true",
    )
    worker.add_argument(
        "-n",
        "--worker-name",
        required=True,
        help="The name of the new worker",
        type=str,
    )
    stubs = init_subparsers.add_parser("stubs", help="Generates worker stubs with UV.")
    stubs.add_argument(
        "--worker-directory",
        help="Directory where to search for workers.",
        type=str,
        default=Path("./tkr") / "workers",
    )
    stubs.add_argument(
        "--api-file-name",
        help="File location where to generate api to. Relative to the worker directory",
        type=str,
        default=Path("./src/api/api.py"),
    )
    return parser


def _gen_worker(worker_name: str, worker_dir: Path, *, external: bool = False) -> None:
    base_dir = worker_dir / worker_name
    base_dir.mkdir(exist_ok=True)
    with Path.open(base_dir / "README.md", "w+", encoding="utf-8") as fh:
        fh.write(f"# {worker_name} \n")
    with Path.open(base_dir / "pyproject.toml", "w+", encoding="utf-8") as fh:
        fh.write(python_worker_workspace_pyproject(worker_name, external=external))
    api_dir = base_dir / "api"
    src_dir = base_dir / "src"
    api_dir.mkdir(exist_ok=True)
    with Path.open(api_dir / "pyproject.toml", "w+", encoding="utf-8") as fh:
        fh.write(python_worker_pyproject(worker_name, kind="api"))
    with Path.open(api_dir / "README.md", "w+", encoding="utf-8") as fh:
        fh.write(f"# {worker_name}-api \n")
    src_dir.mkdir(exist_ok=True)
    if external:
        with Path.open(src_dir / f"{worker_name}.tsp", "w+", encoding="utf-8") as fh:
            fh.write(external_worker_idl(worker_name))
        return
    with Path.open(src_dir / "main.py", "w+", encoding="utf-8") as fh:
        fh.write(python_worker_main(worker_name))
    with Path.open(src_dir / "pyproject.toml", "w+", encoding="utf-8") as fh:
        fh.write(python_worker_pyproject(worker_name, kind="src"))
    with Path.open(src_dir / "README.md", "w+", encoding="utf-8") as fh:
        fh.write(f"# {worker_name}-src \n")


def _gen_stubs(worker_directory: Path, stubs_name: str) -> None:
    uv_path = shutil.which("uv")
    if uv_path is None:
        msg = "uv is required to use this feature."
        raise TierkreisError(msg)
    for worker in worker_directory.iterdir():
        if not worker.is_dir():
            continue
        if (idl := next(worker.glob("*.tsp"), None)) is not None:
            namespace = Namespace.from_spec_file(idl)
            namespace.write_stubs(idl.parent / stubs_name)
        else:
            subprocess.run(
                [uv_path, "run", "--active", "src/main.py", "--stubs-path", stubs_name],
                cwd=worker,
            )


def run_args(args: argparse.Namespace) -> None:
    """Run the project initialization according to the args."""
    if args.init_type == "project":
        project_dir = Path(args.project_directory)
        if (project_dir / "tkr").exists():
            if not (project_dir / "tkr").is_dir():
                msg = f"Project directory {project_dir / 'tkr'} already exists and is not a directory."
                raise TierkreisError(msg)
            if (
                not input(
                    f"Project directory {project_dir / 'tkr'} already exists. Continue? (y/n): "
                )
                .lower()
                .strip()[:1]
                == "y"
            ):
                return
        worker_name = "example_worker"
        worker_dir = Path(args.worker_directory)
        if not worker_dir.is_absolute():
            worker_dir = project_dir / worker_dir
        graphs_dir = Path(args.graphs_directory)
        if not graphs_dir.is_absolute():
            graphs_dir = project_dir / graphs_dir
        worker_dir.mkdir(exist_ok=True, parents=True)
        _gen_worker(worker_name, worker_dir)
        graphs_dir.mkdir(exist_ok=True, parents=True)
        with Path.open(graphs_dir / "main.py", "w+", encoding="utf-8") as fh:
            fh.write(default_graph(worker_name))
        os.environ["TKR_DIR"] = str(args.default_checkpoint_directory)
        _gen_stubs(worker_dir, "./src/api/api.py")
        print(f"""Successfully generated project in '{project_dir / "tkr"}'.
              
To run the sample graph use "python -m tkr.graphs.main".
Or import the function into a top level script with:
              
from tkr.graphs.main import main
main()
              
It is highly recommended to include the newly created structure
to your project definition e.g. pyproject.toml.
Keep in mind that workers are independent of your graph code.
""")
    elif args.init_type == "worker":
        Path(args.worker_directory).mkdir(exist_ok=True, parents=True)
        _gen_worker(args.name, Path(args.worker_directory), external=args.external)
    elif args.init_type == "stubs":
        _gen_stubs(Path(args.worker_directory), args.api_file_name)


class TierkreisInitCli:
    """Tierkeirs cli for the `init` subcommand.

    Used to initialize tkr projects..
    """

    @staticmethod
    def add_subcommand(
        main_parser: argparse._SubParsersAction,
    ) -> None:
        """Add the init subcommand."""
        parser = main_parser.add_parser(
            "init",
            description="Initializes the tierkreis project resources",
            help="Initializes the tierkreis project resources. Run `tkr init --help`"
            " for more information.",
        )
        parser = parse_args(parser)
        parser.set_defaults(func=TierkreisInitCli.execute)

    @staticmethod
    def execute(args: argparse.Namespace) -> None:
        """Execute the init subcommand."""
        run_args(args)
