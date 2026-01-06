import signal
from sys import argv
from tierkreis.controller.data.graph import GraphData
from tierkreis_visualization.app_config import (
    App,
    StorageType,
    graph_data_lifespan,
    dev_lifespan,
)
from tierkreis_visualization.config import CONFIG
from tierkreis_visualization.storage import (
    file_storage_fn,
    from_graph_data_storage_fn,
    graph_data_storage_fn,
)
from tierkreis_visualization.routers.frontend import assets
from tierkreis_visualization.routers.workflows import router as workflows_router
from tierkreis_visualization.routers.frontend import router as frontend_router


def transform_to_sigkill(signum, frame):
    signal.raise_signal(signal.SIGKILL)


def get_app(app_lifespan=None):
    app = App(lifespan=app_lifespan) if app_lifespan else App()
    app.include_router(workflows_router, prefix="/api/workflows")
    app.mount("/assets/", assets, name="frontend_assets")
    app.include_router(frontend_router)

    return app


def get_filestorage_app(app_lifespan=None):
    app = get_app(app_lifespan)
    app.state.get_storage_fn = file_storage_fn(CONFIG.tierkreis_path)
    app.state.storage_type = StorageType.FILESTORAGE
    return app


def get_dev_app():
    # Terminate websocket connection on uvicorn dev reload
    signal.signal(signal.SIGTERM, transform_to_sigkill)

    return get_filestorage_app(dev_lifespan)


def get_graph_data_app():
    app = get_app(graph_data_lifespan)
    graph_specifier = argv[1] if len(argv) > 1 else CONFIG.graph_specifier

    if graph_specifier is None:
        return app

    app.state.get_storage_fn = graph_data_storage_fn(graph_specifier)[0]
    app.state.storage_type = StorageType.GRAPHDATA
    return app


def app_from_graph_data(graph_data: GraphData):
    app = get_app()
    app.state.get_storage_fn = from_graph_data_storage_fn(graph_data)
    app.state.storage_type = StorageType.GRAPHDATA
    return app
