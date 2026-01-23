from abc import ABC, abstractmethod
from pathlib import Path

from pydantic import BaseModel
from tierkreis.consts import WORKER_CACHE


class TKRDependency(ABC, BaseModel):
    type: str

    @abstractmethod
    def install(
        self, worker_name: str, target_dir: Path, worker_cache: Path = WORKER_CACHE
    ) -> None: ...

    """Install a worker called `worker_name` into the `target_dir`.
    
    Any cached files should be installed in the dir `worker_cache / self.type`."""
