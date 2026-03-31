"""Models for type structures used for building the graph."""

from dataclasses import dataclass
from inspect import isclass
from typing import (
    Any,
    Callable,
    Literal,
    Protocol,
    Union,
    cast,
    dataclass_transform,
    get_args,
    get_origin,
    overload,
    runtime_checkable,
)

from typing_extensions import TypeIs

from tierkreis.controller.data.core import (
    NodeIndex,
    PortID,
    RestrictedNamedTuple,
    ValueRef,
)
from tierkreis.controller.data.graph import GraphData
from tierkreis.controller.data.types import PType

from pydantic import (
    BaseModel,
    model_serializer,
    SkipValidation,
)

TKR_PORTMAPPING_FLAG = "__tkr_portmapping__"


@runtime_checkable
class PNamedModel(RestrictedNamedTuple[PType], Protocol):
    """A struct whose members are restricted to being PTypes.

    E.g. used to specify multiple outputs in Python worker code.
    """


@dataclass_transform()
def portmapping[T: PNamedModel](cls: type[T]) -> type[T]:
    setattr(cls, TKR_PORTMAPPING_FLAG, True)
    return cls


PModel = PNamedModel | PType
OpaqueType = Literal


@dataclass
class TKR[T: PModel]:
    node_index: NodeIndex
    port_id: PortID

    def value_ref(self) -> ValueRef:
        return (self.node_index, self.port_id)


@runtime_checkable
class TNamedModel(RestrictedNamedTuple[TKR[PType] | None], Protocol):
    """A struct whose members are restricted to being references to PTypes.

    E.g. in graph builder code these are outputs of tasks.
    """


TModel = TNamedModel | TKR


@overload
def is_portmapping(o: PModel) -> TypeIs[PNamedModel]: ...
@overload
def is_portmapping(o: type[PModel]) -> TypeIs[type[PNamedModel]]: ...
@overload
def is_portmapping(o: TModel) -> TypeIs[TNamedModel]: ...
@overload
def is_portmapping(o: type[TModel]) -> TypeIs[type[TNamedModel]]: ...
def is_portmapping(
    o,
) -> (
    TypeIs[type[PNamedModel]]
    | TypeIs[PNamedModel]
    | TypeIs[TNamedModel]
    | TypeIs[type[TNamedModel]]
):
    origin = get_origin(o)
    if origin is not None:
        return is_portmapping(origin)
    return hasattr(o, TKR_PORTMAPPING_FLAG)


def is_tnamedmodel(o) -> TypeIs[type[TNamedModel]]:  # noqa: ANN001 inherited from get_origin
    origin = get_origin(o)
    if origin is not None:
        return is_tnamedmodel(origin)
    return isclass(o) and issubclass(o, TNamedModel)


def dict_from_pmodel(pmodel: PModel) -> dict[PortID, PType]:
    if is_portmapping(pmodel):
        return pmodel._asdict()

    return {"value": pmodel}


def annotations_from_pmodel(pmodel: type) -> dict[PortID, Any]:
    if is_portmapping(pmodel):
        origin = get_origin(pmodel) or pmodel
        return origin.__annotations__

    return {"value": pmodel}


def dict_from_tmodel(tmodel: TModel) -> dict[PortID, ValueRef]:
    if isinstance(tmodel, TNamedModel):
        return {k: (v.node_index, v.port_id) for k, v in tmodel._asdict().items() if v}

    return {"value": (tmodel.node_index, tmodel.port_id)}


def model_fields(model: type[PModel] | type[TModel]) -> list[str]:
    if is_portmapping(model):
        return getattr(model, "_fields")

    if is_tnamedmodel(model):
        return getattr(model, "_fields")

    return ["value"]


def init_tmodel[T: TModel](tmodel: type[T], input_fn: Callable[[str], ValueRef]) -> T:
    fields = {k: input_fn(k) for k in model_fields(tmodel)}
    if is_tnamedmodel(tmodel):
        o = get_origin(tmodel)
        model = tmodel if not is_tnamedmodel(o) else o
        args: list[TKR] = []
        for field, val in fields.items():
            param = model.__annotations__[field.replace("-*", "")]
            if get_origin(param) == Union:
                param = next(x for x in get_args(param) if x)
            args.append(param(*val))
        return cast("T", model(*args))
    (ref,) = fields.values()
    return tmodel(*ref)


class Workflow[Inputs: TModel, Outputs: TModel](BaseModel):
    data: GraphData
    outputs_type: SkipValidation[type[Outputs]]

    def __class_getitem__(cls, item):
        #try:
        inp, out = item
        if get_origin(inp) == TKR:
            inp = TKR
        if get_origin(out) == TKR:
            out = TKR
        item = (inp, out)
        #except ValueError:
        #    pass
        return super().__class_getitem__(item)

    @model_serializer
    def serialize_model(self) -> dict[str, object]:
        # Just serialize the underlying GraphData, not this frontend/builder type.
        # We'll reinstate the Workflow wrapper and outputs_type on deserialization.
        return self.data.model_dump(mode="json")
