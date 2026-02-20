import pytest

from tierkreis.controller.data.graph import GraphData
from tierkreis.exceptions import TierkreisError


def test_only_one_output() -> None:
    with pytest.raises(TierkreisError):
        g = GraphData()
        g.output({"one": g.const(1)})
        g.output({"two": g.const(2)})
