from copy import copy
from dataclasses import dataclass
from inspect import isclass
from typing import (
    Any,
    Callable,
    Mapping,
    NamedTuple,
    Protocol,
    overload,
    runtime_checkable,
)

from tierkreis.controller.data.core import EmptyModel
from tierkreis.controller.data.models import (
    TKR,
    TModel,
    TNamedModel,
    dict_from_tmodel,
    init_tmodel_fields,
)
from tierkreis.controller.data.types import PType
from tierkreis.controller.data.graph import GraphData, ValueRef, reindex_inputs


@dataclass
class TList[T: TModel]:
    """A list of models."""

    _value: T


@runtime_checkable
class Function[Out](TNamedModel, Protocol):
    @property
    def namespace(self) -> str: ...

    @staticmethod
    def out() -> type[Out]: ...


@dataclass
class TypedGraphRef[Ins: TModel, Outs: TModel]:
    graph_ref: ValueRef
    outputs_type: type[Outs]
    inputs_type: type[Ins]


class LoopOutput(TNamedModel, Protocol):
    @property
    def should_continue(self) -> TKR[bool]: ...


def script(script_name: str, input: TKR[bytes]) -> Function[TKR[bytes]]:
    class exec_script(NamedTuple):
        input: TKR[bytes]

        @staticmethod
        def out() -> type[TKR[bytes]]:
            return TKR[bytes]

        @property
        def namespace(self) -> str:
            return script_name

    return exec_script(input=input)


class GraphBuilder[Inputs: TModel, Outputs: TModel]:
    outputs_type: type
    inputs: Inputs

    def __init__(
        self,
        inputs_type: type[Inputs] = EmptyModel,
        outputs_type: type[Outputs] = EmptyModel,
    ):
        self.data = GraphData()
        self.inputs_type = inputs_type
        self.outputs_type = outputs_type
        self.inputs = init_tmodel_fields(self.inputs_type, self.data.input)

    def get_data(self) -> GraphData:
        return self.data

    def ref(self) -> TypedGraphRef[Inputs, Outputs]:
        return TypedGraphRef((-1, "body"), self.outputs_type, self.inputs_type)

    def outputs(self, outputs: Outputs):
        self.data.output(inputs=dict_from_tmodel(outputs))

    def embed[A: TModel, B: TModel](self, other: "GraphBuilder[A, B]", inputs: A) -> B:
        if other.data.graph_output_idx is None:
            raise ValueError("Can only embed graphs with an output node defined.")
        ins: Mapping[str, ValueRef] = dict_from_tmodel(inputs)

        node_map: dict[int, int] = {}  # other node idx -> self node idx
        port_map: dict[int, Mapping[str, ValueRef]] = {}

        def reindex(vr: ValueRef) -> ValueRef:
            idx, port = vr
            if idx in port_map:
                return port_map[idx][port]
            if idx in node_map:
                return (node_map[idx], port)
            add_node(idx)
            return (node_map[idx], port) if idx in node_map else port_map[idx][port]

        def add_node(idx: int):
            node = other.data.nodes[idx]
            if node.type == "input":
                assert idx not in port_map
                port_map[idx] = {node.name: ins[node.name]}
                return
            assert idx not in node_map
            assert node.type != "output"
            new_node_def = copy(node)
            reindex_inputs(new_node_def, reindex)
            new_node_def.outputs = {}

            func = self.data.add(new_node_def)
            node_map[idx] = func("dummy_port")[0]

        outputs = other.data.nodes[other.data.graph_output_idx].inputs
        return init_tmodel_fields(other.outputs_type, lambda p: reindex(outputs[p]))

    def const[T: PType](self, value: T) -> TKR[T]:
        idx, port = self.data.const(value)
        return TKR[T](idx, port)

    def ifelse[A: PType, B: PType](
        self, pred: TKR[bool], if_true: TKR[A], if_false: TKR[B]
    ) -> TKR[A] | TKR[B]:
        idx, port = self.data.if_else(
            pred.value_ref(), if_true.value_ref(), if_false.value_ref()
        )("value")
        return TKR(idx, port)

    def eifelse[A: PType, B: PType](
        self, pred: TKR[bool], if_true: TKR[A], if_false: TKR[B]
    ) -> TKR[A] | TKR[B]:
        idx, port = self.data.eager_if_else(
            pred.value_ref(), if_true.value_ref(), if_false.value_ref()
        )("value")
        return TKR(idx, port)

    def _graph_const[A: TModel, B: TModel](
        self, graph: "GraphBuilder[A, B]"
    ) -> TypedGraphRef[A, B]:
        idx, port = self.data.const(graph.data.model_dump())
        return TypedGraphRef[A, B](
            graph_ref=(idx, port),
            outputs_type=graph.outputs_type,
            inputs_type=graph.inputs_type,
        )

    def task[Out: TModel](self, f: Function[Out]) -> Out:
        name = f"{f.namespace}.{f.__class__.__name__}"
        ins = dict_from_tmodel(f)
        idx, _ = self.data.func(name, ins)("dummy")
        OutModel = f.out()
        return init_tmodel_fields(OutModel, lambda p: (idx, p))

    @overload
    def eval[A: TModel, B: TModel](self, body: TypedGraphRef[A, B], a: A) -> B: ...
    @overload
    def eval[A: TModel, B: TModel](self, body: "GraphBuilder[A, B]", a: A) -> B: ...
    def eval[A: TModel, B: TModel](
        self, body: "GraphBuilder[A,B] | TypedGraphRef", a: Any
    ) -> Any:
        if isinstance(body, GraphBuilder):
            body = self._graph_const(body)

        idx, _ = self.data.eval(body.graph_ref, dict_from_tmodel(a))("dummy")
        return init_tmodel_fields(body.outputs_type, lambda p: (idx, p))

    @overload
    def loop[A: TModel, B: LoopOutput](
        self, body: TypedGraphRef[A, B], a: A, name: str | None = None
    ) -> B: ...
    @overload
    def loop[A: TModel, B: LoopOutput](
        self, body: "GraphBuilder[A, B]", a: A, name: str | None = None
    ) -> B: ...
    def loop[A: TModel, B: LoopOutput](
        self,
        body: "TypedGraphRef[A, B] |GraphBuilder[A, B]",
        a: A,
        name: str | None = None,
    ) -> B:
        if isinstance(body, GraphBuilder):
            body = self._graph_const(body)

        g = body.graph_ref
        idx, _ = self.data.loop(g, dict_from_tmodel(a), "should_continue", name)(
            "dummy"
        )
        return init_tmodel_fields(body.outputs_type, lambda p: (idx, p))

    def _unfold_list[T: PType](self, ref: TKR[list[T]]) -> TList[TKR[T]]:
        ins = (ref.node_index, ref.port_id)
        idx, _ = self.data.func("builtins.unfold_values", {"value": ins})("dummy")
        return TList(TKR[T](idx, "*"))

    def _fold_list[T: PType](self, refs: TList[TKR[T]]) -> TKR[list[T]]:
        value_ref = (refs._value.node_index, refs._value.port_id)
        idx, _ = self.data.func("builtins.fold_values", {"values_glob": value_ref})(
            "dummy"
        )
        return TKR[list[T]](idx, "value")

    def _map_fn_single_in[A: PType, B: TModel](
        self, aes: TKR[list[A]], body: Callable[[TKR[A]], B]
    ) -> "TList[B]":
        tlist = self._unfold_list(aes)
        return TList(body(TKR(tlist._value.node_index, "*")))

    def _map_fn_single_out[A: TModel, B: PType](
        self, aes: TList[A], body: Callable[[A], TKR[B]]
    ) -> TKR[list[B]]:
        return self._fold_list(TList(body(aes._value)))

    def _map_graph_full[A: TModel, B: TModel](
        self, aes: TList[A], body: TypedGraphRef[A, B]
    ) -> TList[B]:
        ins = dict_from_tmodel(aes._value)
        idx, _ = self.data.map(body.graph_ref, ins)("x")

        return TList(init_tmodel_fields(body.outputs_type, lambda s: (idx, s + "-*")))

    @overload
    def map[A: PType, B: TNamedModel](
        self,
        body: (
            Callable[[TKR[A]], B] | TypedGraphRef[TKR[A], B] | "GraphBuilder[TKR[A], B]"
        ),
        aes: TKR[list[A]],
    ) -> TList[B]: ...

    @overload
    def map[A: TNamedModel, B: PType](
        self,
        body: (
            Callable[[A], TKR[B]] | TypedGraphRef[A, TKR[B]] | "GraphBuilder[A, TKR[B]]"
        ),
        aes: TList[A],
    ) -> TKR[list[B]]: ...

    @overload
    def map[A: TNamedModel, B: TNamedModel](
        self, body: TypedGraphRef[A, B] | "GraphBuilder[A, B]", aes: TList[A]
    ) -> TList[B]: ...

    @overload
    def map[A: PType, B: PType](
        self,
        body: (
            Callable[[TKR[A]], TKR[B]]
            | TypedGraphRef[TKR[A], TKR[B]]
            | "GraphBuilder[TKR[A], TKR[B]]"
        ),
        aes: TKR[list[A]],
    ) -> TKR[list[B]]: ...

    def map(
        self, body: TypedGraphRef | Callable | "GraphBuilder", aes: TKR | TList
    ) -> Any:
        if isinstance(body, GraphBuilder):
            body = self._graph_const(body)

        if isinstance(body, Callable):
            if isinstance(aes, TList):
                return self._map_fn_single_out(aes, body)
            elif isinstance(aes, TKR):
                return self._map_fn_single_in(aes, body)

        if isinstance(aes, TKR):
            aes = self._unfold_list(aes)

        out = self._map_graph_full(aes, body)

        if not isclass(body.outputs_type) or not issubclass(
            body.outputs_type, TNamedModel
        ):
            out = self._fold_list(out)

        return out
