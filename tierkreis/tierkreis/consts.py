from pathlib import Path


PACKAGE_PATH = Path(__file__).parent.parent
TESTS_PATH = PACKAGE_PATH / "tests"
TKR_DIR_KEY = "TKR_DIR"
WORKERS_DIR = PACKAGE_PATH / ".." / "tierkreis_workers"
