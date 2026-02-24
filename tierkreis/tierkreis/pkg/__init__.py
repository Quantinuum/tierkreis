"""Manage dependencies for a Tierkreis project in the standard directory layout.

Usually the target directory is the `workers_external` directory of the project.
Each `TKRDependency` adds a folder to the target directory containing the worker
 code/executable. (Alternatively on systems where copying/moving large numbers of inodes
 is slow the TKRDependency can add a symlink.) Each `TKRDependency` has at its disposal
 a directory `worker_cache / TKRDependency.type` for any caching it needs.
"""

import logging
from pathlib import Path
from shutil import rmtree

from tierkreis.consts import WORKER_CACHE
from tierkreis.pkg.base import TKRDependency

logger = logging.getLogger(__name__)


def install_dependencies(
    deps: dict[str, TKRDependency],
    target_dir: Path,
    worker_cache: Path = WORKER_CACHE,
) -> None:
    """Install the dependencies in `deps` into `target_dir`."""
    for worker_name, dep in deps.items():
        dep.install(worker_name, target_dir, worker_cache)


def remove_dependencies(deps: list[str], target_dir: Path) -> None:
    """Remove the worker directories listed in `deps` from the `target_dir`.

    Assumes that the worker directory is a symlink or a folder.
    """
    for dep in deps:
        worker_dir = target_dir / dep
        if worker_dir.is_symlink():
            Path.unlink(worker_dir)
        elif worker_dir.is_dir():
            rmtree(worker_dir)
        else:
            logger.warning(
                "Expected %s to be a symlink or directory. Taking no action.",
                worker_dir,
            )


def clear_cache(worker_cache: Path = WORKER_CACHE) -> None:
    """Remove all cached files used to install external worker dependencies."""
    rmtree(worker_cache)
