from pydantic import BaseModel, Field

from tierkreis.controller.data.core import PortID
from tierkreis.controller.data.graph import NodeDef
from tierkreis.controller.data.location import Loc


class NodeRunData(BaseModel):
    """Data required to run a node.

    :fields:
        node_location (Loc): The location of the node to run.
        node (NodeDef): The node definition to run.
        output_list (list[PortID]): The list of output port ids for the node.
    """

    node_location: Loc
    node: NodeDef
    output_list: list[PortID]


class WalkResult(BaseModel):
    """Dataclass to keep track of the nodes we encounter during the walk.

    :fields:
        inputs_ready (list[NodeRunData]): A list of nodes that now have all inputs ready
            and therefore can be started.
        started (list[Loc]): A list of locations that have been started (on this walk).
        errored (list[Loc]): A list of locations that have encountered an error.
    """

    inputs_ready: list[NodeRunData]
    started: list[Loc]
    errored: list[Loc] = Field(default_factory=list[Loc])
    breaks: Loc | None = None

    def extend(self, walk_result: "WalkResult") -> None:
        """Extend a current walk result with an existing one.

        Simply extends all three list fields accordingly.

        :param walk_result: The walk_result to update self with.
        :type walk_result: WalkResult
        """
        self.inputs_ready.extend(walk_result.inputs_ready)
        self.started.extend(walk_result.started)
        self.errored.extend(walk_result.errored)
        if self.breaks is None:
            self.breaks = walk_result.breaks
