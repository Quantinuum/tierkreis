import logging
from sys import argv, exit

from tierkreis import Worker

logger = logging.getLogger(__name__)
worker = Worker("failing_worker")


@worker.task()
def fail() -> int:
    logger.error("Raising an error now...")
    msg = "Worker failed!"
    raise ValueError(msg)


@worker.task()
def wont_fail() -> int:
    return 0


@worker.task()
def exit_code_1() -> int:
    exit(1)


if __name__ == "__main__":
    worker.app(argv)
