"""Base class for dependencies of Tierkreis workers."""

from abc import ABC, abstractmethod
from pathlib import Path

from pydantic import BaseModel

from tierkreis.consts import WORKER_CACHE


class TKRDependency(BaseModel, ABC):
    """A worker dependency for a Tierkreis project.

    :fields:
        type (str): The type of the dependency. Used for the cache.
    """

    type: str

    @abstractmethod
    def install(
        self,
        worker_name: str,
        target_dir: Path,
        worker_cache: Path = WORKER_CACHE,
    ) -> None:
        """Install a worker called `worker_name` into the `target_dir`.

        The `TKRDependency` has at its disposal a directory
         `worker_cache / TKRDependency.type` for any caching it needs.

        :param worker_name: The name of the worker.
        :type worker_name: str
        :param target_dir: The target directory to install the worker into.
        :type target_dir: pathlib.Path
        :param worker_cache: The worker cache directory, defaults to WORKER_CACHE
        :type worker_cache: pathlib.Path, optional
        """
        ...
