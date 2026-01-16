from pathlib import Path
from uuid import UUID

from tierkreis.controller.data.core import PortID
from tierkreis.controller.data.location import Loc
from tierkreis.controller.storage.filestorage import ControllerFileStorage


def test_read_worker_call_args_resolves_paths(tmp_path: Path) -> None:
    workflow_id = UUID(int=1)
    storage = ControllerFileStorage(workflow_id, tierkreis_directory=tmp_path)
    storage.clean_graph_files()

    node_location = Loc("-.N0")
    other_node = Loc("-.N1")

    storage.write_worker_call_args(
        node_location=node_location,
        function_name="example.fn",
        inputs={PortID("x"): (other_node, PortID("value"))},
        output_list=[PortID("out")],
    )

    call_args = storage.read_worker_call_args(node_location)

    assert call_args.logs_path == tmp_path / str(workflow_id) / str(node_location) / "logs"
    assert call_args.logs_path is not None and call_args.logs_path.is_absolute()
    assert call_args.output_dir.is_absolute()
    assert call_args.done_path.is_absolute()
    assert call_args.error_path.is_absolute()
    assert all(p.is_absolute() for p in call_args.inputs.values())
    assert all(p.is_absolute() for p in call_args.outputs.values())

