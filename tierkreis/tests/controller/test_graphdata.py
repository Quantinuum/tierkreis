import pytest

from tierkreis.controller.data.graph import GraphData
from tierkreis.exceptions import TierkreisError


def test_only_one_output() -> None:
    g = GraphData()
    g.output({"one": g.const(1)})
    with pytest.raises(TierkreisError):
        g.output({"two": g.const(2)})
