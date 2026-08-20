import logging

from tierkreis import Worker

logger = logging.getLogger(__name__)
worker = Worker("error_worker")


@worker.task()
def fail() -> str:
    msg = "I refuse!"
    raise ValueError(msg)
    return "I failed to refuse"
