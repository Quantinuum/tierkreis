from typing import Literal, Any
from pydantic import BaseModel
from tierkreis.controller.data.location import Loc

NodeStatus = Literal["Not started", "Started", "Error", "Finished"]


class PyNode(BaseModel):
    id: Loc
    status: NodeStatus
    function_name: str
    node_type: str
    node_location: str = ""
    value: Any | None = None
    started_time: str
    finished_time: str


class PyEdge(BaseModel):
    from_node: Loc
    from_port: str
    to_node: Loc
    to_port: str
    value: Any | None = None
    conditional: bool = False
