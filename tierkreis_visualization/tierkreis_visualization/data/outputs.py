import array
from tierkreis.controller.storage.protocol import ControllerStorage
from tierkreis.controller.data.location import Loc
from tierkreis.exceptions import TierkreisError


def outputs_from_loc(
    storage: ControllerStorage, loc: Loc, port_name: str
) -> str | None:
    try:
        raw_bytes = storage.read_output(loc, port_name)
        try:
            return raw_bytes.decode()
        except UnicodeDecodeError:
            pass
        try:
            arr = array.array("d")
            arr.frombytes(raw_bytes)
            return str(arr.tolist())
        except (UnicodeDecodeError, ValueError):
            pass
        return str(raw_bytes.decode("latin-1"))

    except (FileNotFoundError, TierkreisError, UnicodeDecodeError):
        return None
