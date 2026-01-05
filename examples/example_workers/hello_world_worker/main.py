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
worker = Worker("hello_world_worker")


@worker.task()
def greet(greeting: str, subject: str) -> str:
    logger.info("%s %s", greeting, subject)
    return greeting + subject


if __name__ == "__main__":
    worker.app(argv)
