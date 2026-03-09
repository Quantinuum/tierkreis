from sys import argv
from time import sleep

from tierkreis import Worker

worker = Worker("sleep_worker")


@worker.task()
def sleep_and_return[T](*, output: T) -> T:
    sleep(10)
    return output


if __name__ == "__main__":
    worker.app(argv)
