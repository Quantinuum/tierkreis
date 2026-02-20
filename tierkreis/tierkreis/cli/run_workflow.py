import logging
import uuid
from pathlib import Path

from tierkreis.controller import run_graph
from tierkreis.controller.data.graph import GraphData
from tierkreis.controller.data.types import PType
from tierkreis.controller.executor.shell_executor import ShellExecutor
from tierkreis.controller.executor.uv_executor import UvExecutor
from tierkreis.controller.storage.filestorage import ControllerFileStorage
from tierkreis.storage import read_outputs

logger = logging.getLogger(__name__)


def run_workflow(
    graph: GraphData,
    inputs: dict[str, PType],
    name: str | None = None,
    run_id: int | None = None,
    log_level: int | str = logging.INFO,
    registry_path: Path | None = None,
    print_output: bool = False,
    use_uv_worker: bool = False,
    n_iterations: int = 10000,
    polling_interval_seconds: float = 0.1,
) -> None:
    """Run a workflow."""
    logger.setLevel(log_level)
    workflow_id = uuid.uuid4() if run_id is None else uuid.UUID(int=run_id)
    logger.info("Workflow ID is %s", workflow_id)
    storage = ControllerFileStorage(workflow_id, name=name, do_cleanup=True)
    if registry_path is None:
        registry_path = Path(__file__).parent
    if use_uv_worker:
        executor = UvExecutor(registry_path=registry_path, logs_path=storage.logs_path)
    else:
        executor = ShellExecutor(registry_path, storage.workflow_dir)

    logger.info("Starting workflow at location: %s", storage.logs_path)

    run_graph(
        storage,
        executor,
        graph,
        inputs,
        n_iterations,
        polling_interval_seconds,
    )
    if print_output:
        read_outputs(graph, storage)
