from sys import argv

import uvicorn


def start() -> None:
    uvicorn.run(
        "tierkreis_visualization.app:get_filestorage_app", timeout_graceful_shutdown=10
    )


def dev() -> None:
    uvicorn.run("tierkreis_visualization.app:get_dev_app", reload=True)


def graph() -> None:
    """Visualize a computation graph in a web browser.

    Entrypoint for the project script tkr-vis-graph.
    """
    reload_path = argv[1].split(":", 1)[0]
    uvicorn.run(
        "tierkreis_visualization.app:get_graph_data_app",
        reload=True,
        reload_includes=reload_path,
    )


def openapi() -> None:
    from tierkreis_visualization.openapi import generate_openapi
    from tierkreis_visualization.app import get_filestorage_app

    generate_openapi(get_filestorage_app())


if __name__ == "__main__":
    start()
