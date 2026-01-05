# /// script
# requires-python = ">=3.12"
# dependencies = ["pydantic", "tierkreis", "ruff"]
#
# [tool.uv.sources]
# tierkreis = { path = "../../../tierkreis", editable = true }
# ///
import logging
from sys import argv
from tierkreis import Worker

logger = logging.getLogger(__name__)
worker = Worker("error_worker")


@worker.task()
def fail() -> str:
    raise Exception("I refuse!")
    return "I failed to refuse"


if __name__ == "__main__":
    worker.app(argv)
