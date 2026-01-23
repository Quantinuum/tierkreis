from os import unlink
from pathlib import Path
from shutil import rmtree
from tierkreis.consts import WORKER_CACHE
from tierkreis.pkg.base import TKRDependency


def install_dependencies(deps: dict[str, TKRDependency], target_dir: Path):
    """Install the dependencies in `deps` into `target_dir`."""
    for worker_name, dep in deps.items():
        dep.install(worker_name, target_dir)


def remove_dependencies(deps: list[str], target_dir: Path):
    for dep in deps:
        unlink(target_dir / dep)


def clear_cache():
    """Remove all cached files used to install external worker depenencies."""
    rmtree(WORKER_CACHE)
