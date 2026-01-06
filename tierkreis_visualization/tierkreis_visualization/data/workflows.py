import os
from datetime import datetime
from uuid import UUID

from pydantic import BaseModel

from tierkreis.controller.data.location import Loc
from tierkreis_visualization.app_config import StorageType
from tierkreis_visualization.config import CONFIG
from tierkreis_visualization.storage import file_storage_fn


class WorkflowDisplay(BaseModel):
    id: UUID
    id_int: int
    name: str | None
    start_time: str
    errors: list[str]


def get_workflows(storage_type: StorageType) -> list[WorkflowDisplay]:
    if storage_type == StorageType.GRAPHDATA:
        return [
            WorkflowDisplay(
                id=UUID(int=0),
                id_int=0,
                name="tmp",
                start_time=datetime.now().isoformat(),
                errors=[],
            )
        ]
    return get_workflows_from_disk()


def get_workflows_from_disk() -> list[WorkflowDisplay]:
    folders = os.listdir(CONFIG.tierkreis_path)
    folders.sort()
    workflows: list[WorkflowDisplay] = []
    for folder in folders:
        try:
            id = UUID(folder)
            storage = file_storage_fn(CONFIG.tierkreis_path)(id)
            metadata = storage.read_metadata(Loc(""))
            name = metadata["name"] or "workflow"
            start = metadata.get("start_time", datetime.now().isoformat())
            errors = [x for x in storage.read_errors(Loc()).split("\n") if x]
            errors = list(set(errors))
            workflows.append(
                WorkflowDisplay(
                    id=id, id_int=int(id), name=name, start_time=start, errors=errors
                )
            )
        except (TypeError, ValueError):
            continue

    return workflows
