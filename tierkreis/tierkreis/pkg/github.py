"""Worker dependency from github."""

import shutil
import subprocess
from pathlib import Path
from typing import override

from tierkreis.consts import WORKER_CACHE
from tierkreis.exceptions import TierkreisError
from tierkreis.pkg.base import TKRDependency


class GitHubDependency(TKRDependency):
    """A TKRDependency that installs a worker from a github repository.

    :fields:
        account (str): The github account to clone from.
        repo (str): The github repository to clone from.
        subdirectory (str): The subdirectory in the repository to use as the worker.
        branch (str): The branch to clone from. Defaults to "main".
    """

    type: str = "github"

    account: str
    repo: str
    subdirectory: str
    branch: str = "main"

    def cache_subdir(self, worker_cache: Path) -> Path:
        """Get the worker cache subdirectory.

        :param worker_cache: The base directory for worker caches.
        :type worker_cache: pathlib.Path
        :return: The subdirectory path within the worker cache.
        :rtype: pathlib.Path
        """
        return worker_cache / "github" / self.account / self.repo / self.branch

    @override
    def install(
        self,
        worker_name: str,
        target_dir: Path,
        worker_cache: Path = WORKER_CACHE,
    ) -> None:
        cache_dir = self.cache_subdir(worker_cache)
        cache_dir.mkdir(exist_ok=True, parents=True)
        git_path = shutil.which("git")
        if git_path is None:
            msg = "git is required to use github dependencies."
            raise TierkreisError(msg)
        git_dir = cache_dir / ".git"
        git_url = f"https://github.com/{self.account}/{self.repo}"
        if not git_dir.exists():
            subprocess.run([git_path, "clone", git_url, "."], cwd=cache_dir, check=True)

        subprocess.run([git_path, "restore", "."], cwd=cache_dir, check=True)
        subprocess.run([git_path, "clean", "-f"], cwd=cache_dir, check=True)
        subprocess.run([git_path, "checkout", self.branch], cwd=cache_dir, check=True)
        subprocess.run([git_path, "pull", "--rebase"], cwd=cache_dir, check=True)

        worker_dir = target_dir / worker_name
        if worker_dir.exists():
            worker_dir.unlink()

        Path.symlink_to(
            cache_dir / self.subdirectory,
            worker_dir,
            target_is_directory=True,
        )
