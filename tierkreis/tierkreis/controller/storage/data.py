from datetime import datetime

from pydantic import BaseModel, Field


class WorkflowMetaData(BaseModel):
    workflow_id: str
    tierkreis_version: str
    user_id: str
    name: str | None = None
    start_time: str | None = None  # can we store workflows to be run later?
    completion_time: str | None = None
    execution_count: int = 0

    @property
    def duration(self) -> float | None:
        if self.start_time and self.completion_time:
            start = datetime.fromisoformat(self.start_time)
            end = datetime.fromisoformat(self.completion_time)
            return (end - start).total_seconds()
        return None


class ExecutorDebugData(BaseModel):
    """Debug data from executors."""

    executor: str
    launch_command: str
    job_id: str | None = None
    env: dict[str, str] = Field(default_factory=dict)
    packages: list[str] = Field(default_factory=list)


class NodeDebugData(BaseModel):
    """Debug data for graph nodes.

    Currently only used for loop_nodes
    Storage implementations should decide which are applicable.
    """

    loop_loc: str | None = None
