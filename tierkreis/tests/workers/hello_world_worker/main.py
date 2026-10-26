import logging
from sys import argv
from time import sleep

from tierkreis import Worker

logger = logging.getLogger(__name__)
worker = Worker("hello_world_worker")


@worker.task()
def greet(greeting: str, subject: str) -> str:
    logger.info("%s %s", greeting, subject)
    return greeting + subject


@worker.task()
def sleep_and_return(*, output: str, delay_seconds: float) -> str:
    sleep(delay_seconds)
    return output


if __name__ == "__main__":
    worker.app(argv)
