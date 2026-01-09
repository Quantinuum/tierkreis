from importlib.util import module_from_spec, spec_from_file_location
from pathlib import Path
from sys import argv
import sys
from typing import Callable
from uuid import UUID

from tierkreis.controller.storage.filestorage import ControllerFileStorage
from tierkreis.controller.storage.graphdata import GraphDataStorage
from tierkreis.controller.storage.protocol import ControllerStorage
from tierkreis_core import GraphData


def file_storage_fn(tkr_dir: Path) -> Callable[[UUID], ControllerStorage]:
    def inner(workflow_id: UUID):
        return ControllerFileStorage(
            workflow_id=workflow_id, tierkreis_directory=tkr_dir
        )

    return inner


def graph_data_storage_fn(
    graph_specifier: str,
) -> tuple[Callable[[UUID], ControllerStorage], Path]:
    graph_specifier = argv[1]
    mod_path, var = graph_specifier.rsplit(":", 1)
    spec = spec_from_file_location("tkr_tmp.graph", mod_path)

    if spec is None:
        raise ValueError(f"File is not a Python module: {mod_path}")

    module = module_from_spec(spec)
    sys.modules["tkr_tmp.graph"] = module
    loader = spec.loader

    if loader is None:
        raise ValueError("Could not get loader from module.")

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
