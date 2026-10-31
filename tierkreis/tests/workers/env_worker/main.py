import os
from sys import argv

from tierkreis import Worker

worker = Worker("env_worker")


@worker.task()
def read_env(name: str) -> str:
    value = os.environ.get(name, "")
    return value


if __name__ == "__main__":
    worker.app(argv)
