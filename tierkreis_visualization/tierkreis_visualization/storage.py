import os
import sys
from importlib.util import module_from_spec, spec_from_file_location
from pathlib import Path
from typing import Callable
from uuid import UUID

from tierkreis.controller.data.graph import GraphData
from tierkreis.controller.storage.filestorage import ControllerFileStorage
from tierkreis.controller.storage.graphdata import GraphDataStorage
from tierkreis.controller.storage.protocol import ControllerStorage


def file_storage_fn(tkr_dir: Path) -> Callable[[UUID], ControllerStorage]:
    def inner(workflow_id: UUID):
        return ControllerFileStorage(
            workflow_id=workflow_id,
            tierkreis_directory=tkr_dir,
        )

    return inner


def graph_data_storage_fn(
    graph_specifier: str,
) -> tuple[Callable[[UUID], ControllerStorage], Path]:
    mod_path, var = graph_specifier.rsplit(":", 1)

    # ensure relative imports can be found
    cwd = os.getcwd()
    if cwd not in sys.path:
        sys.path.append(cwd)
    file_dir = os.path.dirname(mod_path)
    if file_dir not in sys.path:
        sys.path.append(file_dir)

    spec = spec_from_file_location(
        "tkr_tmp.graph", mod_path, submodule_search_locations=[os.getcwd()]
    )
    if spec is None:
        msg = f"File is not a Python module: {mod_path}"
        raise ValueError(msg)

    module = module_from_spec(spec)
    sys.modules["tkr_tmp.graph"] = module
    loader = spec.loader

    if loader is None:
        msg = "Could not get loader from module."
        raise ValueError(msg)

    loader.exec_module(module)
    graph = getattr(module, var).data

    def inner(workflow_id: UUID) -> ControllerStorage:
        return GraphDataStorage(UUID(int=0), graph=graph)

    return inner, Path(mod_path)


def from_graph_data_storage_fn(
    graph_data: GraphData,
) -> Callable[[UUID], ControllerStorage]:
    def inner(workflow_id: UUID) -> ControllerStorage:
        return GraphDataStorage(UUID(int=0), graph=graph_data)

    return inner
