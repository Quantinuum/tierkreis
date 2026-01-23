from os import symlink
from pathlib import Path
import subprocess
from typing import Literal
from tierkreis.consts import WORKER_CACHE
from tierkreis.pkg.base import TKRDependency


class GitHubDependency(TKRDependency):
    type: Literal["github"] = "github"

    account: str
    repo: str
    subdirectory: str

    branch: str = "main"

    @property
    def cache_dir(self):
        return WORKER_CACHE / "github" / self.account / self.repo / self.branch

    def install(self, worker_name: str, target_dir: Path):
        cache_dir = self.cache_dir
        cache_dir.mkdir(exist_ok=True, parents=True)

        git_dir = cache_dir / ".git"
        git_url = f"https://github.com/{self.account}/{self.repo}"
        if not git_dir.exists():
            subprocess.run(["git", "clone", git_url, "."], cwd=cache_dir)

        subprocess.run(["git", "clean", "-f"], cwd=cache_dir)
        subprocess.run(["git", "checkout", self.branch], cwd=cache_dir)
        subprocess.run(["git", "pull", "--rebase"], cwd=cache_dir)

        worker_dir = target_dir / worker_name
        if worker_dir.exists():
            worker_dir.unlink()

        symlink(cache_dir / self.subdirectory, worker_dir, target_is_directory=True)
