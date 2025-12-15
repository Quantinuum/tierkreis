import inspect
from typing import Any, Callable, get_type_hints
from pydantic import create_model, ValidationError, Field
from tierkreis.controller.data.models import PModel
from tierkreis.exceptions import TierkreisError


def validate_func_args(func: Callable[..., PModel], kwargs: dict[str, Any]) -> None:
    type_hints = get_type_hints(func)
    signature = inspect.signature(func)
    fields: dict[str, Any] = {}

    accepts_var_kwargs = False

    for param_name, param in signature.parameters.items():
        if param.kind == inspect.Parameter.VAR_KEYWORD:
            accepts_var_kwargs = True
            continue
        if param.kind == inspect.Parameter.VAR_POSITIONAL:
            continue
        annotation = type_hints.get(param_name, Any)
        if param.default == inspect.Parameter.empty:
            default = Field(...)
        else:
            default = param.default

        fields[param_name] = (annotation, default)

    DynamicModel = create_model(f"{func.__name__}_Validator", **fields)
    if accepts_var_kwargs:
        DynamicModel.model_config["extra"] = "allow"
    else:
        DynamicModel.model_config["extra"] = "forbid"

    try:
        DynamicModel(**kwargs)
    except ValidationError as e:
        raise TierkreisError("Invalid inputs provide to worker function.") from e
