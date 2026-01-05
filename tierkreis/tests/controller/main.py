# /// script
# requires-python = ">=3.12"
# dependencies = ["pydantic", "tierkreis"]
#
# [tool.uv.sources]
# tierkreis = { path = "../../../tierkreis", editable = true }
# ///
from pathlib import Path
from time import sleep
from sys import argv

from tierkreis import Worker

worker = Worker("tests_worker")


@worker.task()
def sleep_and_return[T](*, output: T) -> T:
    sleep(10)
    return output


def main() -> None:
    node_definition_path = argv[1]
    worker.run(Path(node_definition_path))


if __name__ == "__main__":
    main()
