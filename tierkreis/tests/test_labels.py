import pytest

from tierkreis.labels import Labels


def test_instantiate_raises() -> None:
    with pytest.raises(RuntimeError):
        Labels()
