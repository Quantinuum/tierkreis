import array

from tierkreis.controller.data.location import Loc
from tierkreis.controller.storage.protocol import ControllerStorage
from tierkreis.exceptions import TierkreisError
from tierkreis_visualization.data.models import NodeInputs


def outputs_from_loc(
    storage: ControllerStorage,
    loc: Loc,
    port_name: str,
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


def task_inputs(storage: ControllerStorage, loc: Loc) -> list[NodeInputs]:
    try:
        call_args = storage.read_worker_call_args(loc)
        inputs = []
        for port, path in call_args.inputs.items():
            if len(parts := path.parts) != 4:  # should be <uuid>/<loc>/outputs/<port>
                raise TierkreisError(f"Invalid input path: {path}")
            inputs.append(
                NodeInputs(port=port, from_node=Loc(parts[1]), from_port=parts[3])
            )
        return inputs
    except (FileNotFoundError, TierkreisError):
        return []
