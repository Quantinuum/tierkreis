import pytest


def pytest_addoption(parser: pytest.Parser) -> None:
    parser.addoption(
        "--optional",
        action="store_true",
        default=False,
        help="run optional tests",
    )


def pytest_configure(config: pytest.Config) -> None:
    config.addinivalue_line("markers", "optional: mark test as optional to run")


def pytest_collection_modifyitems(
    config: pytest.Config,
    items: list[pytest.Item],
) -> None:
    if config.getoption("--optional"):
        return
    skip_slow = pytest.mark.skip(reason="need --optional option to run")
    for item in items:
        if "optional" in item.keywords:
            item.add_marker(skip_slow)
