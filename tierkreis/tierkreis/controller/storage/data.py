from dataclasses import dataclass, field
from uuid import UUID


@dataclass
class WorkflowMetaData:
    workflow_id: UUID
    tierkreis_version: str
    user_id: str
    start_time: str | None = None  # can we store workflows to be run later?
    completion_time: str | None = None
    execution_count: int = 0

    @property
    def duration(self) -> str | None:
        return None


@dataclass
class ExecutorData:
    executor: str
    launch_command: str
    job_id: str | None = None
    env: dict[str, str] = field(default={})
    packages: list[str] = field(default=[])
