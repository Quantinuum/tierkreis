"""Tests for the public runtime facade."""

import pytest
from pydantic import BaseModel

from tierkreis import runtime


def memory_config() -> dict[str, object]:
    """Return the smallest useful JSON-compatible runtime configuration."""
    return {
        "asset_storage": {"memory": {"type": "Memory"}},
        "executors": {"memory": {"type": "Memory", "output_storage_name": "memory"}},
        "runtime_state": {"type": "Memory"},
        "default_storage_name": "memory",
        "default_executor_name": "memory",
        "logging_config": None,
    }


class RuntimeConfigModel(BaseModel):
    """Pydantic representation used to exercise the model-dump adapter."""

    asset_storage: dict[str, dict[str, object]]
    executors: dict[str, dict[str, object]]
    runtime_state: dict[str, object]
    default_storage_name: str
    default_executor_name: str
    logging_config: None


def test_runtime_api_is_public() -> None:
    """The supported facade exposes the Rust runtime entry points."""
    assert runtime.Runtime is not None
    assert hasattr(runtime.Runtime, "get_raw_outputs")
    assert runtime.RuntimeConfig is not None
    assert callable(runtime.new_default)
    assert callable(runtime.new_from_config)
    assert callable(runtime.load_runtime_config)


@pytest.mark.asyncio
@pytest.mark.parametrize("as_model", [False, True])
async def test_runtime_config_accepts_mapping_or_pydantic_model(as_model: bool) -> None:
    """Runtime configuration uses the same Serde schema from Python."""
    config = memory_config()
    value = RuntimeConfigModel.model_validate(config) if as_model else config

    instance = await runtime.new_from_config(value)
    with instance:
        pass


@pytest.mark.asyncio
async def test_runtime_config_error_identifies_nested_field() -> None:
    """Invalid configuration errors retain the failing Serde path."""
    config = memory_config()
    config["runtime_state"] = {"type": "Unknown"}

    with pytest.raises(ValueError, match="runtime_state"):
        await runtime.new_from_config(config)
