from pathlib import Path

import pytest

from tests.controller.loop_graphdata import loop_multiple_acc, loop_multiple_acc_untyped
from tierkreis import Runtime
from tierkreis.controller.data.graph import GraphData

return_value = [
    {"acc1": x, "acc2": y, "acc3": z}
    for x, y, z in zip(range(1, 7), range(2, 13, 2), range(3, 19, 3), strict=True)
]

params: list[tuple[GraphData, list[dict[str, int]], str, int]] = [
    (
        loop_multiple_acc_untyped(),
        return_value,
        "multi_acc",
        9,
    ),
    (
        loop_multiple_acc().data,
        return_value,
        "multi_acc",
        9,
    ),
]
ids = [
    "loop_multiple_acc_untyped",
    "loop_multiple_acc",
]

storage_kinds = ["sqlite", "memory"]


@pytest.mark.parametrize("storage_kind", storage_kinds)
@pytest.mark.parametrize(("graph", "output", "name", "workflow_id"), params, ids=ids)
def test_read_loop_trace(
    storage_kind: str,
    graph: GraphData,
    output: list[dict[str, int]],
    name: str,
    workflow_id: int,
    tmp_path: Path,
) -> None:
    del workflow_id
    runtime = (
        Runtime()
        if storage_kind == "memory"
        else Runtime.sqlite(tmp_path / "runtime.sqlite", tmp_path / "assets")
    )
    with runtime:
        uploaded_id = runtime.upload_workflow(name, graph)
        run_id = runtime.start_workflow(uploaded_id, {})
        assert runtime.wait(run_id, timeout=30) == "Completed"
        state = runtime.get_workflow_state(run_id)

        loop_idx = graph.named_nodes["my_loop"]
        loop_state = state.nodes[f"N{loop_idx}"]
        assert loop_state.loop_index == len(output) - 1

        actual_output = []
        for iteration in range(len(output)):
            prefix = f"N{loop_idx}.L{iteration}."
            output_nodes = [
                node
                for location, node in state.nodes.items()
                if location.startswith(prefix) and "should_continue" in node.outputs
            ]
            assert len(output_nodes) == 1
            actual_output.append({
                key: value
                for key, value in output_nodes[0].outputs.items()
                if key != "should_continue"
            })

        assert actual_output == output
