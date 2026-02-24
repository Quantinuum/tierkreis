"""Core Tierkreis Label type."""


class Labels:
    """Special port labels used by builtin functions."""

    def __init__(self) -> None:
        msg = "Do not instantiate"
        raise RuntimeError(msg)

    THUNK = "thunk"
    VALUE = "value"
    VARIANT_VALUE = "variant_value"
    CONTINUE = "continue"
    BREAK = "break"
