from pathlib import Path
from types import TracebackType
from typing import Any, Self
from uuid import UUID

class NodeStateSnapshot:
    location: str
    status: str
    scheduled_time: str | None
    queued_time: str | None
    running_time: str | None
    complete_time: str | None
    cancelled_time: str | None
    error_time: str | None
    outputs: dict[str, Any | None]
    condition: bool | None
    loop_index: int | None
    map_completed: list[bool] | None
    error: str | None
    error_detail: str | None
    task_handle: str | None

class WorkflowStateSnapshot:
    workflow_id: UUID
    run_id: UUID
    attempt: int
    name: str | None
    status: str
    started_time: str | None
    metadata: dict[str, str]
    nodes: dict[str, NodeStateSnapshot]

class Runtime:
    def __init__(self) -> None: ...
    @staticmethod
    def sqlite(
        database_path: Path,
        asset_dir: Path,
        *,
        executor: str = "memory",
    ) -> Runtime: ...
    def upload_workflow(self, name: str, workflow: Any) -> UUID: ...
    def start_workflow(self, workflow_id: UUID, inputs: Any) -> UUID: ...
    def wait(
        self,
        run_id: UUID,
        *,
        attempt: int | None = None,
        timeout: float | None = None,
    ) -> str: ...
    def status(self, run_id: UUID, *, attempt: int | None = None) -> str: ...
    def get_outputs(self, run_id: UUID, *, attempt: int | None = None) -> Any: ...
    def get_workflow_state(
        self,
        run_id: UUID,
        *,
        attempt: int | None = None,
    ) -> WorkflowStateSnapshot: ...
    def close(self) -> None: ...
    def __enter__(self) -> Self: ...
    def __exit__(
        self,
        exc_type: type[BaseException] | None,
        exc_value: BaseException | None,
        traceback: TracebackType | None,
    ) -> None: ...

def run_workflow(name: str, workflow: Any, inputs: Any) -> Any: ...
