import uvicorn

from tierkreis.builder import GraphBuilder
from tierkreis_core import GraphData

from tierkreis_visualization.app import app_from_graph_data


def visualize_graph(graph_data: GraphData | GraphBuilder) -> None:
    """Visualize a computation graph in a web browser.

    :param graph_data: The computation graph to visualize.
    :type graph_data: GraphData | GraphBuilder
    """
    if isinstance(graph_data, GraphBuilder):
        graph_data = graph_data.get_data()
    app = app_from_graph_data(graph_data)
    uvicorn.run(app)
