"""Typed graph builder for Tierkreis workflows."""

from __future__ import annotations

from collections.abc import Callable
from copy import copy
from dataclasses import dataclass
from inspect import isclass
from typing import (
    Any,
    Mapping,
    NamedTuple,
    Protocol,
    overload,
    runtime_checkable,
)

from tierkreis.controller.data.core import EmptyModel
from tierkreis.controller.data.graph import GraphData, ValueRef, reindex_inputs
from tierkreis.controller.data.models import (
    TKR,
    TModel,
    TNamedModel,
    dict_from_tmodel,
    init_tmodel,
    Workflow,
)
from tierkreis.controller.data.types import PType


@dataclass
class TList[T: TModel]:
    """A list of models."""

    _value: T


@runtime_checkable
class Function[Out](TNamedModel, Protocol):
    """A worker function type.

    :abstract:
    """

    @property
    def namespace(self) -> str:
        """The namespace name.

        :return: The namespace name.
        :rtype: str
        """
        ...

    @staticmethod
    def out() -> type[Out]:
        """Return the output type of the function.

        :return: The output type.
        :rtype: type[Out]
        """
        ...


@dataclass
class TypedGraphRef[Ins: TModel, Outs: TModel]:
    """A typed tierkreis graph.

    :attr graph_ref: The graph reference.
    :attr outputs_type: The output type of the graph.
    """

    graph_ref: TKR[Workflow[Ins, Outs]]
    outputs_type: type[Outs]


class LoopOutput(TNamedModel, Protocol):
    """Protocol for loop output models to ensure should continue."""

    @property
    def should_continue(self) -> TKR[bool]:
        """The loop continuation port.

        :return: The continuation port value.
        :rtype: TKR[bool]
        """
        ...


def script(script_name: str, script_input: TKR[bytes]) -> Function[TKR[bytes]]:
    """Add a script to the graph.

    A shell script or binary with a single input and output.
    Inputs are provided from the standard input and outputs to the standard output.

    :param script_name: The name of the script.
    :type script_name: str
    :param script_input: The input to the script.
    :type script_input: TKR[bytes]
    :return: The output of the script.
    :rtype: Function[TKR[bytes]]
    """

    class exec_script(NamedTuple):  # noqa: N801
        input: TKR[bytes]

        @staticmethod
        def out() -> type[TKR[bytes]]:
            return TKR[bytes]

        @property
        def namespace(self) -> str:
            return script_name

    return exec_script(input=script_input)


class Graph[Inputs: TModel, Outputs: TModel]:
    """Class to construct typed workflow graphs.

    :attr data: The underlying graph data.
    :attr inputs_type: The input type of the graph.
    :attr inputs: The inputs to the graph.
    :attr outputs_type: The output type of the graph.
    """

    inputs_type: type[Inputs]
    outputs_type: type[Outputs]
    inputs: Inputs

    def __init__(
        self,
        inputs_type: type[Inputs] = EmptyModel,
        outputs_type: type[Outputs] = EmptyModel,
    ) -> None:
        self.data = GraphData()
        self.inputs_type = inputs_type
        self.outputs_type = outputs_type
        self.inputs = init_tmodel(self.inputs_type, self.data.input)

    def ref(self) -> TypedGraphRef[Inputs, Outputs]:
        """Return a reference of the typed graph.

        :return: The ref of the typed graph.
        :rtype: TypedGraphRef[Inputs, Outputs]
        """
        return TypedGraphRef(TKR(-1, "body"), self.outputs_type)

    def finish_with_outputs(self, outputs: Outputs) -> Workflow[Inputs, Outputs]:
        """Set output nodes of a graph.

        :param outputs: The output nodes.
        :type outputs: Outputs
        """
        self.data.output(inputs=dict_from_tmodel(outputs))
        return Workflow(self.data, self.outputs_type)

    def embed[A: TModel, B: TModel](
        self, other_fg: Workflow[A, B], inputs: A, outputs_type: type[B]
    ) -> B:
        other = other_fg.data
        if other.graph_output_idx is None:
            raise ValueError("Can only embed graphs with an output node defined.")
        ins: Mapping[str, ValueRef] = dict_from_tmodel(inputs)

        node_map: dict[int, int] = {}  # other node idx -> self node idx
        port_map: dict[int, Mapping[str, ValueRef]] = {}

        def reindex(vr: ValueRef) -> ValueRef:
            idx, port = vr
            if idx in port_map:
                return port_map[idx][port]
            if idx not in node_map:
                node = other.nodes[idx]
                if node.type == "input":
                    port_map[idx] = {node.name: ins[node.name]}
                    return port_map[idx][port]
                assert node.type != "output"
                new_node_def = copy(node)
                reindex_inputs(new_node_def, reindex)
                new_node_def.outputs = {}

                func = self.data.add(new_node_def)
                node_map[idx] = func("dummy_port")[0]
            return (node_map[idx], port)

        outputs = other.nodes[other.graph_output_idx].inputs
        return init_tmodel(other_fg.outputs_type, lambda p: reindex(outputs[p]))

    def const[T: PType](self, value: T) -> TKR[T]:
        """Add a constant node to the graph.

        :return: The constant value.
        :rtype: TKR[T]
        """
        # Workflow exists at build-time only; erase the types at runtime:
        idx, port = self.data.const(
            value.data if isinstance(value, Workflow) else value
        )
        return TKR[T](idx, port)

    def ifelse[A: PType, B: PType](
        self,
        pred: TKR[bool],
        if_true: TKR[A],
        if_false: TKR[B],
    ) -> TKR[A] | TKR[B]:
        """Add an if-else node to the graph.

        This will be evaluated lazily.
        The values can be returned from an eval node or another graph.

        :param pred: The predicate value.
        :type pred: TKR[bool]
        :param if_true: The value if the predicate is true.
        :type if_true: TKR[A]
        :param if_false: The value if the predicate is false.
        :type if_false: TKR[B]
        :return: The outputs of the if-else expression.
        :rtype: TKR[A] | TKR[B]
        """
        idx, port = self.data.if_else(
            pred.value_ref(),
            if_true.value_ref(),
            if_false.value_ref(),
        )("value")
        return TKR(idx, port)

    def eifelse[A: PType, B: PType](
        self,
        pred: TKR[bool],
        if_true: TKR[A],
        if_false: TKR[B],
    ) -> TKR[A] | TKR[B]:
        """Add an eager if-else node to the graph.

        This will be evaluated eagerly.
        The values can be returned from an eval node or another graph.

        :param pred: The predicate value.
        :type pred: TKR[bool]
        :param if_true: The value if the predicate is true.
        :type if_true: TKR[A]
        :param if_false: The value if the predicate is false.
        :type if_false: TKR[B]
        :return: The outputs of the if-else expression.
        :rtype: TKR[A] | TKR[B]
        """
        idx, port = self.data.eager_if_else(
            pred.value_ref(),
            if_true.value_ref(),
            if_false.value_ref(),
        )("value")
        return TKR(idx, port)

    def _graph_const[A: TModel, B: TModel](
        self,
        graph: Workflow[A, B],
    ) -> TypedGraphRef[A, B]:
        # TODO @philipp-seitz: Turn this into a public method?
        return TypedGraphRef(
            self.const(graph),
            graph.outputs_type,
        )

    def task[Out: TModel](self, func: Function[Out]) -> Out:
        """Add a worker task node to the graph.

        :param func: The worker function.
        :type func: Function[Out]
        :return: The outputs of the task.
        :rtype: Out
        """
        name = f"{func.namespace}.{func.__class__.__name__}"
        inputs = dict_from_tmodel(func)
        idx, _ = self.data.func(name, inputs)("dummy")
        OutModel = func.out()  # noqa: N806
        return init_tmodel(OutModel, lambda p: (idx, p))

    def eval[A: TModel, B: TModel](
        self,
        body: Workflow[A, B] | TypedGraphRef[A, B],
        eval_inputs: A,
    ) -> B:
        """Add a evaluation node to the graph.

        This will evaluate a nested graph with the given inputs.

        :param A the input type of the graph.
        :param B the output type of the graph.
        :param body: The graph to evaluate.
        :param eval_inputs: The inputs to the graph.
        :return: The outputs of the evaluation.
        """
        if isinstance(body, Workflow):
            body = self._graph_const(body)

        idx, _ = self.data.eval(
            body.graph_ref.value_ref(), dict_from_tmodel(eval_inputs)
        )("dummy")
        return init_tmodel(body.outputs_type, lambda p: (idx, p))

    def loop[A: TModel, B: LoopOutput](
        self,
        body: TypedGraphRef[A, B] | Workflow[A, B],
        loop_inputs: A,
        name: str | None = None,
    ) -> B:
        """Add a loop node to the graph.

        This will loop over the given graph until the `should_continue` output is false.
        To trace intermediate values, use the name attribute in conjunction with
        read_loop_trace.

        :param A the input type of the graph.
        :param B the output type of the graph.
        :param body: The graph to loop.
        :param loop_inputs: The inputs to the loop graph.
        :param name: An optional name for the loop.
        :return: The outputs of the loop.
        """
        if isinstance(body, Workflow):
            body = self._graph_const(body)

        idx, _ = self.data.loop(
            body.graph_ref.value_ref(),
            dict_from_tmodel(loop_inputs),
            "should_continue",
            name,
        )(
            "dummy",
        )
        return init_tmodel(body.outputs_type, lambda p: (idx, p))

    def _unfold_list[T: PType](self, ref: TKR[list[T]]) -> TList[TKR[T]]:
        ins = (ref.node_index, ref.port_id)
        idx, _ = self.data.func("builtins.unfold_values", {"value": ins})("dummy")
        return TList(TKR[T](idx, "*"))

    def _fold_list[T: PType](self, refs: TList[TKR[T]]) -> TKR[list[T]]:
        value_ref = (refs._value.node_index, refs._value.port_id)  # noqa: SLF001
        idx, _ = self.data.func("builtins.fold_values", {"values_glob": value_ref})(
            "dummy",
        )
        return TKR[list[T]](idx, "value")

    def _map_fn_single_in[A: PType, B: TModel](
        self,
        map_inputs: TKR[list[A]],
        body: Callable[[TKR[A]], B],
    ) -> TList[B]:
        tlist = self._unfold_list(map_inputs)
        return TList(body(TKR(tlist._value.node_index, "*")))  # noqa: SLF001

    def _map_fn_single_out[A: TModel, B: PType](
        self,
        map_inputs: TList[A],
        body: Callable[[A], TKR[B]],
    ) -> TKR[list[B]]:
        return self._fold_list(TList(body(map_inputs._value)))  # noqa: SLF001

    def _map_graph_full[A: TModel, B: TModel](
        self,
        map_inputs: TList[A],
        body: TypedGraphRef[A, B],
    ) -> TList[B]:
        ins = dict_from_tmodel(map_inputs._value)  # noqa: SLF001
        idx, _ = self.data.map(body.graph_ref.value_ref(), ins)("x")

        return TList(init_tmodel(body.outputs_type, lambda s: (idx, s + "-*")))

    @overload
    def map[A: PType, B: TNamedModel](
        self,
        body: (Callable[[TKR[A]], B] | TypedGraphRef[TKR[A], B] | Workflow[TKR[A], B]),
        map_inputs: TKR[list[A]],
    ) -> TList[B]: ...

    @overload
    def map[A: TNamedModel, B: PType](
        self,
        body: (Callable[[A], TKR[B]] | TypedGraphRef[A, TKR[B]] | Workflow[A, TKR[B]]),
        map_inputs: TList[A],
    ) -> TKR[list[B]]: ...

    @overload
    def map[A: TNamedModel, B: TNamedModel](
        self,
        body: TypedGraphRef[A, B] | Workflow[A, B],
        map_inputs: TList[A],
    ) -> TList[B]: ...

    @overload
    def map[A: PType, B: PType](
        self,
        body: (
            Callable[[TKR[A]], TKR[B]]
            | TypedGraphRef[TKR[A], TKR[B]]
            | Workflow[TKR[A], TKR[B]]
        ),
        map_inputs: TKR[list[A]],
    ) -> TKR[list[B]]: ...

    def map(
        self,
        body: TypedGraphRef | Callable | Workflow,
        map_inputs: TKR | TList,
    ) -> Any:
        """Add a map node to the graph.

        :param body: The graph to map over.
        :param map_inputs: The values to map over.
        :return: The outputs of the map.
        """
        if isinstance(body, Workflow):
            body = self._graph_const(body)

        if isinstance(body, Callable):
            if isinstance(map_inputs, TList):
                return self._map_fn_single_out(map_inputs, body)
            if isinstance(map_inputs, TKR):
                return self._map_fn_single_in(map_inputs, body)

        if isinstance(map_inputs, TKR):
            map_inputs = self._unfold_list(map_inputs)

        out = self._map_graph_full(map_inputs, body)

        if not isclass(body.outputs_type) or not issubclass(
            body.outputs_type,
            TNamedModel,
        ):
            out = self._fold_list(out)

        return out
