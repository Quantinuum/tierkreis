import asyncio

import uvicorn

from tierkreis.builder import Graph, Workflow
from tierkreis.controller.data.graph import GraphData
from tierkreis_visualization.app import app_from_graph_data


def visualize_graph(graph_data: GraphData | Workflow | Graph) -> None:
    """Visualize a computation graph in a web browser.

    :param graph_data: The computation graph to visualize.
    :type graph_data: GraphData | Graph
    """
    # Accessing the .data of a Graph isn't generally good practice but makes sense here.
    if isinstance(graph_data, (Workflow, Graph)):
        graph_data = graph_data.data
    app = app_from_graph_data(graph_data)
    config = uvicorn.Config(app, host="0.0.0.0", port=8000, loop="asyncio")
    server = uvicorn.Server(config)

    # Manually run the serve method, bypassing uvicorn.run's patch logic
    asyncio.run(server.serve())
