"""Manage dependencies for a Tierkreis project in the standard directory layout.

Usually the target directory is the `workers_external` directory of the project."""

import logging
from os import unlink
from pathlib import Path
from shutil import rmtree
from tierkreis.consts import WORKER_CACHE
from tierkreis.pkg.base import TKRDependency

logger = logging.getLogger(__name__)


def install_dependencies(deps: dict[str, TKRDependency], target_dir: Path):
    """Install the dependencies in `deps` into `target_dir`."""
    for worker_name, dep in deps.items():
        dep.install(worker_name, target_dir)


def remove_dependencies(deps: list[str], target_dir: Path):
    """Remove the worker directories listed in `deps` from the `target_dir`.

    Assumes that the worker directory is a symlink or a folder."""

    for dep in deps:
        worker_dir = target_dir / dep
        if worker_dir.is_symlink():
            unlink(worker_dir)
        elif worker_dir.is_dir():
            rmtree(worker_dir)
        else:
            logger.warning(
                f"Expected {worker_dir} to be a symlink or directory."
                " Taking no action."
            )


def clear_cache():
    """Remove all cached files used to install external worker depenencies."""
    rmtree(WORKER_CACHE)
