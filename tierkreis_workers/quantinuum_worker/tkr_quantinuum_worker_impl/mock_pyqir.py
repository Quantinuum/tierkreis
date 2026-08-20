from sys import modules
from unittest.mock import Mock


def mock_pyqir() -> None:
    """Mock any pyqir modules we run into.

    We don't need pyqir and it doesn't have an installation candidate on some platforms.
    To make this work you need to add the following to your pyproject.toml:
    ```
    [tool.uv]
    override-dependencies = [
        "pyqir; sys_platform == 'never'",
    ]
    ```


    """

    modules["pyqir"] = Mock()
    modules["pytket.qir"] = Mock()
    modules["pytket.qir.conversion"] = Mock()
    modules["pytket.qir.conversion.api"] = Mock()
    modules["pytket.qir.conversion.qirgenerator"] = Mock()


try:
    from pytket.extensions.quantinuum.backends.quantinuum import (
        QuantinuumBackend,
    )
except (ImportError, ModuleNotFoundError):
    mock_pyqir()
    from pytket.extensions.quantinuum.backends.quantinuum import (
        QuantinuumBackend,  # noqa: F401
    )
