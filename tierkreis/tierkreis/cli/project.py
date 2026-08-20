"""CLI for project related operations."""

import argparse
import os
import shutil
import subprocess
import sys
from pathlib import Path

from tierkreis.cli.templates import (
    default_graph,
    external_worker_idl,
    inputs_json,
    python_worker_api_pyproject,
    python_worker_impl,
    python_worker_main,
    python_worker_pyproject,
    worker_api_readme,
    worker_impl_init,
    worker_init,
    worker_readme,
)
from tierkreis.exceptions import TierkreisError
from tierkreis.namespace import Namespace


def parse_args(
    parser: argparse.ArgumentParser,
) -> argparse.ArgumentParser:
    """Parse the arguments for the init subcommand."""
    init_subparsers = parser.add_subparsers(
        dest="init_type",
        help="Initialize Tierkreis related structures",
        required=True,
    )
    project = init_subparsers.add_parser(
        "project",
        description="Initialize and manages project wide options. "
        "Make sure to set up a python project first, e.g. by executing `uv init`.",
        help="Initializes a new Tierkreis project and manages project wide options.",
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
        " Defaults to <project_directory>/workers.",
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
        " Defaults to <project_directory>/workers.",
        type=Path,
        default=None,
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
        help="The name of the new worker.",
        type=str,
    )
    stubs = init_subparsers.add_parser("stubs", help="Generates worker stubs with uv.")
    stubs.add_argument(
        "--worker-directory",
        help="Directory where to search for workers.",
        type=Path,
        default=None,
    )
    stubs.add_argument(
        "--api-file-name",
        help="File location where to generate api to. Relative to the worker directory.",
        type=Path,
        default=Path("./api/api.py"),
    )
    return parser


def _gen_worker(worker_name: str, worker_dir: Path, *, external: bool = False) -> None:
    worker_name = worker_name.replace("-", "_")
    base_dir = worker_dir / worker_name
    base_dir.mkdir(exist_ok=True)
    with Path.open(base_dir / "README.md", "w+", encoding="utf-8") as fh:
        fh.write(worker_readme(worker_name))
    with Path.open(base_dir / "pyproject.toml", "w+", encoding="utf-8") as fh:
        fh.write(python_worker_pyproject(worker_name))
    with Path.open(base_dir / "__init__.py", "w+", encoding="utf-8") as fh:
        fh.write(worker_init())
    src_dir = base_dir / f"tkr_{worker_name}_impl"
    src_dir.mkdir(exist_ok=True)
    api_dir = base_dir / "api"
    api_dir.mkdir(exist_ok=True)
    with Path.open(api_dir / "pyproject.toml", "w+", encoding="utf-8") as fh:
        fh.write(python_worker_api_pyproject(worker_name))
    with Path.open(api_dir / "README.md", "w+", encoding="utf-8") as fh:
        fh.write(worker_api_readme(worker_name))
    if external:
        with Path.open(src_dir / f"{worker_name}.tsp", "w+", encoding="utf-8") as fh:
            fh.write(external_worker_idl(worker_name))
        return
    with Path.open(src_dir / "main.py", "w+", encoding="utf-8") as fh:
        fh.write(python_worker_main(worker_name))
    with Path.open(src_dir / "__init__.py", "w+", encoding="utf-8") as fh:
        fh.write(worker_impl_init())
    with Path.open(src_dir / "impl.py", "w+", encoding="utf-8") as fh:
        fh.write(python_worker_impl(worker_name))

    _gen_worker_stubs(worker_dir, "./api/api.py")


def _gen_stubs(worker_directory: Path, stubs_name: str) -> None:
    uv_path = shutil.which("uv")
    if uv_path is None:
        command = [sys.executable]
    else:
        command = [uv_path, "run", "--active"]
    for worker in worker_directory.iterdir():
        if not worker.is_dir():
            continue
        if (idl := next(worker.glob("*/*.tsp"), None)) is not None:
            namespace = Namespace.from_spec_file(idl)
            namespace.write_stubs(idl.parent / stubs_name)
        else:
            worker_command = command + [
                f"tkr_{worker.name}_impl/main.py",
                "--stubs-path",
                stubs_name,
            ]
            subprocess.run(worker_command, cwd=worker, check=True)


def _gen_worker_stubs(worker_directory: Path, stubs_name: str) -> None:
    uv_path = shutil.which("uv")
    if uv_path is None:
        msg = "uv is required to use this feature."
        raise TierkreisError(msg)
    for worker in worker_directory.iterdir():
        if not worker.is_dir():
            continue
        else:
            worker_name = worker.name
            subprocess.run(
                [
                    uv_path,
                    "run",
                    "--active",
                    f"tkr_{worker_name}_impl/main.py",
                    "--stubs-path",
                    stubs_name,
                ],
                cwd=worker,
                check=True,
            )
            subprocess.run(
                [uv_path, "add", "--active", f"tkr-{worker_name}"],
                cwd=worker,
                check=True,
            )
            subprocess.run(
                [
                    uv_path,
                    "add",
                    "--active",
                    "--editable",
                    f"{worker}",
                    "--no-workspace",
                ],
                check=True,
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
                input(
                    f"Project directory {project_dir / 'tkr'} already exists. Continue? (y/n): "
                )
                .lower()
                .strip()[:1]
                != "y"
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
        with Path.open(
            project_dir / "workflow_inputs.json", "w+", encoding="utf-8"
        ) as fh:
            fh.write(inputs_json())
        os.environ["TKR_DIR"] = str(args.default_checkpoint_directory)
        print(f"""Successfully generated project in '{project_dir / "tkr"}'.
              
To run the sample graph use "uv run tkr/graphs/main.py".
Or import the function into a top level script with:
              
from tkr.graphs.main import main
main()
              
It is highly recommended to examine the newly created structure
and how it was added to your project definition e.g. pyproject.toml.
Keep in mind that workers are independent of your graph code.
""")
    elif args.init_type == "worker":
        worker_dir = args.worker_directory
        if worker_dir is None:
            worker_dir = _find_worker_dir()
            if worker_dir is None:
                print(
                    "Could not find sutiable worker directory **/tkr/workers/ "
                    "Please specify it with --worker-directory or create a 'workers' directory in your project."
                )
                return
        else:
            worker_dir = Path(worker_dir)
            worker_dir.mkdir(exist_ok=True, parents=True)
        _gen_worker(args.worker_name, worker_dir, external=args.external)
    elif args.init_type == "stubs":
        if args.worker_directory is None:
            worker_dir = _find_worker_dir()
            if worker_dir is None:
                print(
                    "Could not find sutiable worker directory **/tkr/workers/ "
                    "Please specify it with --worker-directory or create a 'workers' directory in your project."
                )
                return
        else:
            worker_dir = Path(args.worker_directory)
        _gen_stubs(worker_dir, args.api_file_name)


def _find_worker_dir() -> Path | None:
    project_dir = _get_project_root()
    if project_dir is None:
        return None
    print(f"Searching for worker directory in {project_dir}...")
    for worker_dir in project_dir.glob("**/tkr/workers"):
        print(f"checking: {worker_dir}")
        if worker_dir.is_dir():
            print(f"Found worker directory: {worker_dir}")
            return worker_dir
    return None


def _get_project_root() -> Path | None:
    git_command = shutil.which("git")
    if git_command is None:
        print("git was not found. Some commands might not work as expected.")
        return None
    try:
        result = subprocess.run(
            [git_command, "rev-parse", "--show-toplevel"],
            capture_output=True,
            text=True,
            check=True,
        )
        return Path(result.stdout.strip())
    except subprocess.CalledProcessError:
        return None


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
            description="Initializes the Tierkreis project resources",
            help="Initializes the Tierkreis project resources. Run `tkr init --help`"
            " for more information.",
        )
        parser = parse_args(parser)
        parser.set_defaults(func=TierkreisInitCli.execute)

    @staticmethod
    def execute(args: argparse.Namespace) -> None:
        """Execute the init subcommand."""
        run_args(args)
