from contextlib import asynccontextmanager
from datetime import datetime
from enum import Enum
from typing import Callable
from uuid import UUID
import webbrowser
from fastapi import FastAPI
import starlette.datastructures
from tierkreis.controller.storage.filestorage import ControllerFileStorage
from tierkreis.controller.storage.graphdata import GraphDataStorage
from tierkreis.controller.storage.protocol import ControllerStorage
from tierkreis_visualization.openapi import generate_openapi


class StorageType(Enum):
    FILESTORAGE = ControllerFileStorage
    GRAPHDATA = GraphDataStorage


class State(starlette.datastructures.State):
    """Typed App state information."""

    get_storage_fn: Callable[[UUID], ControllerStorage]
    storage_type: StorageType


class App(FastAPI):
    """FastAPI App with custom state."""

    state: State  # pyright: ignore[reportIncompatibleVariableOverride]


@asynccontextmanager
async def dev_lifespan(app: App):
    generate_openapi(app)
    yield


@asynccontextmanager
async def graph_data_lifespan(app: App):
    webbrowser.open("http://localhost:8000/", new=0, autoraise=False)
    yield
