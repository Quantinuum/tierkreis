from dataclasses import dataclass
from pathlib import Path
from tierkreis.controller.executor.shell_executor import ShellExecutor
from tierkreis.controller.executor.uv_executor import UvExecutor
from tierkreis.controller.executor.multiple import MultipleExecutor
from tierkreis.controller.executor.hpc.pjsub import PJSUBExecutor
from tierkreis.controller.executor.hpc.slurm import SLURMExecutor
from tierkreis.exceptions import TierkreisError


@dataclass
class _Executor:
    def run(self, launcher_name: str, worker_call_args_path: Path) -> None:
        raise TierkreisError(
            "Invoked run on _Executor. Please assign a proper executor to all nodes."
        )


Executor = ShellExecutor | UvExecutor | PJSUBExecutor | SLURMExecutor | _Executor
__all__ = [
    "ShellExecutor",
    "UvExecutor",
    "MultipleExecutor",
    "PJSUBExecutor",
    "SLURMExecutor",
    "Executor",
]
