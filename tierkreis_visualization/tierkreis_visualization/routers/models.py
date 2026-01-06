from pydantic import BaseModel
from tierkreis.controller.data.location import Loc
from tierkreis_visualization.data.models import PyNode, PyEdge


class PyGraph(BaseModel):
    nodes: list[PyNode]
    edges: list[PyEdge]


class GraphsResponse(BaseModel):
    graphs: dict[Loc, PyGraph]
