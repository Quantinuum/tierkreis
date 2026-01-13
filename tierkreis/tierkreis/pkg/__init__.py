from pathlib import Path
import subprocess
from tierkreis.pkg.base import TKRDependency


def install_dependencies(deps: dict[str, TKRDependency], target_dir: Path):
    for worker_name, dep in deps.items():
        dep.install(worker_name, target_dir)

    worker_dirs = [x for x in target_dir.iterdir() if x.is_dir()]
    for worker_dir in worker_dirs:
        subprocess.run(
            [
                "uv",
                "run",
                "--stubs-path",
                f"../../api/{worker_dir.stem}",
                "--stubs-namespace",
                worker_dir.stem,
            ],
            cwd=worker_dir,
        )
