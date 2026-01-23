from pathlib import Path
from shutil import rmtree
from tierkreis.consts import WORKER_CACHE
from tierkreis.pkg.base import TKRDependency


def install_dependencies(deps: dict[str, TKRDependency], target_dir: Path):
    for worker_name, dep in deps.items():
        dep.install(worker_name, target_dir)


def clear_cache():
    rmtree(WORKER_CACHE)
