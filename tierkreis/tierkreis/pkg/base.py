from abc import ABC, abstractmethod
from pathlib import Path

from pydantic import BaseModel


class TKRDependency(ABC, BaseModel):
    @abstractmethod
    def install(self, worker_name: str, target_dir: Path) -> None: ...
