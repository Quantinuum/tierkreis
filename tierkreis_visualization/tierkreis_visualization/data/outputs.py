from tierkreis.controller.storage.protocol import ControllerStorage
from tierkreis.controller.data.location import Loc
from tierkreis.exceptions import TierkreisError


def outputs_from_loc(
    storage: ControllerStorage, loc: Loc, port_name: str
) -> str | None:
    try:
        bs = storage.read_output(loc, port_name)
        return bs.decode()
    except (FileNotFoundError, TierkreisError):
        return None
