from datetime import datetime, timedelta
from os import PathLike
from pathlib import Path
from typing import Any, Final, SupportsIndex, final
from uuid import UUID

from _typeshed import Incomplete, SupportsGetItem

DEFAULT_CONFIG_FILE_NAME: Final[str]

@final
class NodeState:
    @property
    def execution_context(self, /) -> str | None: ...
    @property
    def name(self, /) -> str | None: ...
    @property
    def status(self, /) -> str: ...

@final
class NodeStatus:
    Cancelled: Final[NodeStatus]
    Complete: Final[NodeStatus]
    Error: Final[NodeStatus]
    Paused: Final[NodeStatus]
    Queued: Final[NodeStatus]
    Running: Final[NodeStatus]
    Scheduled: Final[NodeStatus]
    def __int__(self, /) -> int: ...

@final
class PyWorkflowInputs: ...

@final
class Runtime:
    def __enter__(self, /) -> None: ...
    def __exit__(
        self, /, exc_type: type, exc_value: BaseException | None, traceback: Any | None
    ) -> None: ...
    async def debug_read_node_states(
        self,
        /,
        run_id: UUID,
        attempt: SupportsIndex,
        locations: SupportsGetItem[int, str],
    ) -> dict[str, NodeState]: ...
    async def get_loop_iterations(
        self,
        /,
        run_id: UUID,
        attempt: SupportsIndex,
        name: str,
        output_name: str | None = None,
    ) -> (
        dict[str, bool | int | float | str | complex | None]
        | bool
        | int
        | float
        | str
        | complex
    ):
        """
        Same as `get_loop_trace_by_name`, but shaped as one dict per iteration
        E.g. `[
            {"output1": val1, "output2": val3},
            {"output1": val2}
        ]`
        """
    async def get_outputs(
        self, run_id: UUID, attempt: SupportsIndex = 0, /
    ) -> (
        dict[str, bool | int | float | str | complex | None]
        | bool
        | int
        | float
        | str
        | complex
    ): ...
    async def get_raw_outputs(
        self, run_id: UUID, attempt: SupportsIndex = 0, /
    ) -> dict[str, bytes]:
        """
        Fetch the serialized outputs of a workflow run without decoding them.
        """
    async def get_summary(
        self, run_id: UUID, attempt: SupportsIndex = 0, /
    ) -> WorkflowSummary: ...
    async def restart_task(
        self,
        /,
        run_id: UUID,
        locations: SupportsGetItem[int, str] | None = None,
        attempt: SupportsIndex = 0,
    ) -> tuple[int, list[str]]: ...
    async def save_workflow(
        self, /, name: str | None, workflow: Incomplete
    ) -> UUID: ...
    async def start_new_run(
        self, /, workflow_id: UUID, inputs: Incomplete, *, profile: str | None = None
    ) -> UUID: ...
    async def unpause_nodes(
        self,
        /,
        run_id: UUID,
        locations: SupportsGetItem[int, str] | None = None,
        attempt: SupportsIndex = 0,
    ) -> list[str]: ...
    async def wait_for(
        self,
        run_id: UUID,
        attempt: SupportsIndex = 0,
        /,
        timeout: SupportsIndex = 604800,
    ) -> None: ...

@final
class RuntimeConfig:
    def set_log_level(self, /, log_level: str) -> None:
        """
        Override the configured logging level.
        """

@final
class WorkflowGraph: ...

@final
class WorkflowSummary:
    @property
    def cancelled_time(self, /) -> datetime | None:
        """
        The time that the workflow was cancelled.
        """
    @property
    def complete_time(self, /) -> datetime | None:
        """
        The time that the workflow completed.
        """
    @property
    def duration(self, /) -> timedelta | None: ...
    @property
    def error_time(self, /) -> datetime | None:
        """
        The time that the workflow errored.
        """
    @property
    def errored_locations(self, /) -> list[str]:
        """
        Locations of nodes that have errored in this run.
        """
    @property
    def name(self, /) -> str | None: ...
    @property
    def paused_locations(self, /) -> list[str]:
        """
        Locations of nodes that are paused at breakpoints in this run.
        """
    @property
    def queued_time(self, /) -> datetime | None:
        """
        The time that the workflow was queued.
        """
    @property
    def started_time(self, /) -> datetime | None:
        """
        The time that the workflow started.
        """

def create_default_config(path: str | PathLike[str] | None = None) -> Path:
    """
    Write a default `RuntimeConfig` to `~/.tierkreis/tierkreis.toml` if one
    does not already exist there. Returns the path to the config file.
    """

def create_default_directories() -> None:
    """
    Create the default Tierkreis directories (`~/.tierkreis/tmp` and
    `~/.tierkreis/assets`) if they do not already exist.
    """

def flush_traces() -> None: ...
def load_runtime_config(path: str | PathLike[str] | None = None) -> RuntimeConfig:
    """
    Load a runtime config, creating the default config when none exists.
    """

async def new_default(profile: str | None = None) -> Runtime: ...
async def new_from_config(config: Incomplete) -> Runtime: ...
async def new_in_memory() -> Runtime: ...
async def new_sqlite_memory() -> Runtime: ...
def serve(
    port: SupportsIndex = 3000,
    host: str = "127.0.0.1",
    config: str | PathLike[str] | None = None,
) -> None: ...
def serve_graph(graph: Any, port: SupportsIndex = 3000) -> None: ...
def __getattr__(name: str) -> Incomplete: ...
