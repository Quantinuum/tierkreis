from tierkreis.controller.executor.in_memory_executor import InMemoryExecutor
from tierkreis.controller.executor.shell_executor import ShellExecutor

from tierkreis.controller.executor.stdinout import StdInOut
from tierkreis.controller.executor.uv_executor import UvExecutor

type BaseExecutor = ShellExecutor | UvExecutor | StdInOut | InMemoryExecutor
