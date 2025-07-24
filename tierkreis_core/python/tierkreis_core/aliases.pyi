"""Type aliases for documentation and typechecking purposes."""

from typing import Mapping, Sequence

from tierkreis_core._tierkreis_core.graph import GraphData

# Identifiers for "ports" out or into nodes in a Graph.
type PortID = str
# Values that are acceptable for use in a Const node of a Graph.
#
# Strictly a subset of types like `tierkreis.PType`.
type Value = (
    int | float | bool | str | bytes | Sequence[Value] | Mapping[str, Value] | GraphData
)
