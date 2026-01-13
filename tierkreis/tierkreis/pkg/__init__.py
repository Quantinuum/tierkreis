from pathlib import Path
from tierkreis.pkg.base import TKRDependency


def install_dependencies(deps: dict[str, TKRDependency], target_dir: Path):
    for worker_name, dep in deps.items():
        dep.install(worker_name, target_dir)
