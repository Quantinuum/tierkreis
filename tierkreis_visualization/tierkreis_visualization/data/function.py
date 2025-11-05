from pydantic import BaseModel
from tierkreis.controller.data.location import Loc
from tierkreis.controller.storage.protocol import ControllerStorage
from tierkreis.exceptions import TierkreisError


class FunctionDefinition(BaseModel):
    has_error: bool = False
    error_message: str | None = None


def get_function_node(storage: ControllerStorage, loc: Loc) -> FunctionDefinition:
    parent = loc.parent()
    if parent is None:
        raise TierkreisError("Func node must have parent.")
    if not storage.node_has_error(loc):
        return FunctionDefinition()
    return FunctionDefinition(has_error=True, error_message=storage.read_errors(loc))
