import pytest

from tests.controller.typed_graphdata import DoublerInput, typed_doubler_plus
from tierkreis.builder import ExecutionContext, Graph
from tierkreis.builtins import iadd
from tierkreis.controller.data.core import EmptyModel
from tierkreis.controller.data.graph import GraphData
from tierkreis.controller.data.models import TKR
from tierkreis.exceptions import TierkreisError


def test_only_one_output() -> None:
    g = GraphData()
    g.output({"one": g.const(1)})
    with pytest.raises(TierkreisError):
        g.output({"two": g.const(2)})


def test_execution_context_scope_and_serialization() -> None:
    graph = Graph()
    left = graph.const(1)
    right = graph.const(2)

    with ExecutionContext("cpu_only"):
        explicit = graph.task(iadd(left, right), context="gpu_large")
        with ExecutionContext("gpu_large"):
            nested = graph.task(iadd(left, right))
        outer = graph.task(iadd(left, right))

    unscoped = graph.task(iadd(left, right))
    metadata = GraphData.model_validate_json(graph.data.model_dump_json()).node_metadata
    assert metadata[explicit.node_index].context == "gpu_large"
    assert metadata[nested.node_index].context == "gpu_large"
    assert metadata[outer.node_index].context == "cpu_only"
    assert unscoped.node_index not in metadata


def test_embed_preserves_task_context() -> None:
    child = Graph(EmptyModel, TKR[int])
    result = child.task(iadd(child.const(1), child.const(2)), context="gpu_large")
    workflow = child.finish_with_outputs(result)
    parent = Graph(EmptyModel, TKR[int])
    parent.embed(workflow, EmptyModel(), TKR[int])
    assert "gpu_large" in {
        metadata.context for metadata in parent.data.node_metadata.values()
    }


def test_map_context_is_inherited_by_callable_body() -> None:
    g = Graph(TKR[list[int]], TKR[list[int]])
    ins = g.map(lambda n: DoublerInput(x=n, intercept=g.const(6)), g.inputs)
    with ExecutionContext("gpu_large"):
        m = g.map(typed_doubler_plus(), ins)
    g.finish_with_outputs(m)

    assert g.data.node_metadata[4].context == "gpu_large"
    # TODO: currently the below will fail, as the context is stored on the map
    # The runtime can resolve this correctly but is this intended behavior?
    # inner = g.data.nodes[3].value.node_metadata
    # assert inner[0].context == "gpu_large"
