import pytest
from tierkreis_core import GraphData


def test_only_one_output():
    with pytest.raises(ValueError):
        g = GraphData()
        g.output({"one": g.const(1)})
        g.output({"two": g.const(2)})
