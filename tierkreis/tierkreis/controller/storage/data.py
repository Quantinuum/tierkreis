from dataclasses import dataclass, field


@dataclass
class WorkflowMetaData:
    workflow_id: str
    tierkreis_version: str
    user_id: str
    name: str | None = None
    start_time: str | None = None  # can we store workflows to be run later?
    completion_time: str | None = None
    execution_count: int = 0

    @property
    def duration(self) -> str | None:
        return None


@dataclass
class ExecutorDebugData:
    """Debug data from executors."""

    executor: str
    launch_command: str
    job_id: str | None = None
    env: dict[str, str] = field(default_factory=lambda: {})
    packages: list[str] = field(default_factory=lambda: [])


@dataclass
class NodeDebugData:
    """Debug data for graph nodes.

    Currently only used for loop_nodes
    Storage implementations should decide which are applicable."""

    loop_loc: str | None = None
