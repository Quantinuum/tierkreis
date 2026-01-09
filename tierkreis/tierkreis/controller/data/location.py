from pathlib import Path
from typing import Optional

from pydantic import BaseModel
from tierkreis_core import Loc, PortID


class WorkerCallArgs(BaseModel):
    function_name: str
    inputs: dict[str, Path]
    outputs: dict[str, Path]
    output_dir: Path
    done_path: Path
    error_path: Path
    logs_path: Optional[Path]


OutputLoc = tuple[Loc, PortID]
