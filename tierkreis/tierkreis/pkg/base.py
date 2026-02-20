from abc import ABC, abstractmethod
from pathlib import Path

from pydantic import BaseModel

from tierkreis.consts import WORKER_CACHE


class TKRDependency(ABC, BaseModel):
    type: str

    @abstractmethod
    def install(
        self,
        worker_name: str,
        target_dir: Path,
        worker_cache: Path = WORKER_CACHE,
    ) -> None: ...

    """Install a worker called `worker_name` into the `target_dir`.

    The `TKRDependency` has at its disposal a directory `worker_cache / TKRDependency.type` for any caching it needs."""
