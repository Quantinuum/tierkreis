import logging

from tierkreis import Worker

logger = logging.getLogger(__name__)
worker = Worker("hello_world_worker")


@worker.task()
def greet(greeting: str, subject: str) -> str:
    logger.info("%s %s", greeting, subject)
    return greeting + subject
