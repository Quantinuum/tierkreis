import os
import sys

import uvicorn

from tierkreis_visualization.config import CONFIG, TKR_GRAPH_SPECIFIER_KEY
from tierkreis_visualization.storage import graph_data_storage_fn


def get_graph_specifier() -> str | None:
    if TKR_GRAPH_SPECIFIER_KEY in os.environ:
        return os.environ[TKR_GRAPH_SPECIFIER_KEY]
    return sys.argv[1] if len(sys.argv) > 1 else CONFIG.graph_specifier


def start() -> None:
    uvicorn.run(
        "tierkreis_visualization.app:get_filestorage_app",
        timeout_graceful_shutdown=10,
    )


def dev() -> None:
    uvicorn.run("tierkreis_visualization.app:get_dev_app", reload=True)


def graph() -> None:
    """Visualize a computation graph in a web browser.

    Entrypoint for the project script tkr-vis-graph.
    """

    graph_specifier = get_graph_specifier()
    if graph_specifier is None:
        print(
            "No graph specifier provided."
            "Please provide a graph specifier in the form <PATH_TO_PYTHON_FILE>:<VARIABLE_CONTAINING_GRAPH> as a command line argument.",
            file=sys.stderr,
        )
        sys.exit(1)
    try:
        # Make sure we can load the module before spawning the server
        _ = graph_data_storage_fn(graph_specifier)
    except Exception as e:
        msg: str = """Failed to load graph data for visualization.
Potential causes include an invalid graph specifier or an error in the graph code itself."""
        raise RuntimeError(msg) from e
    uvicorn.run(
        "tierkreis_visualization.app:get_graph_data_app",
        reload=True,
        reload_includes=graph_specifier.split(":", 1)[0],
    )


def openapi() -> None:
    from tierkreis_visualization.app import get_filestorage_app
    from tierkreis_visualization.openapi import generate_openapi

    generate_openapi(get_filestorage_app())


if __name__ == "__main__":
    start()
