from typing import Literal, Any
from pydantic import BaseModel
from tierkreis.controller.data.location import Loc

NodeStatus = Literal["Not started", "Started", "Error", "Finished"]
NodeType = Literal[
    "function", "ifelse", "map", "eval", "loop", "eifelse", "const", "output", "input"
]


class PyNode(BaseModel):
    id: Loc
    status: NodeStatus
    function_name: str
    node_type: NodeType
    node_location: str = ""
    outputs: list[str]
    value: str | None = None
    started_time: str
    finished_time: str


class PyEdge(BaseModel):
    from_node: Loc
    from_port: str
    to_node: Loc
    to_port: str
    value: str | None = None
    conditional: bool = False
