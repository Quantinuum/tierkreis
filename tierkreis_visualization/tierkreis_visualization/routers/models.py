from pydantic import BaseModel

from tierkreis.controller.data.location import Loc
from tierkreis_visualization.data.models import PyEdge, PyNode


class PyGraph(BaseModel):
    nodes: list[PyNode]
    edges: list[PyEdge]


class GraphsResponse(BaseModel):
    graphs: dict[Loc, PyGraph]


class RuntimeMetadata(BaseModel):
    version: str
