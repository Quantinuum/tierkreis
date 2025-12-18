import logging
from pathlib import Path

LOGGER_NAME = "TKR"


def set_tkr_logger(file_name: Path, level: int | str = logging.INFO) -> None:
    logger = logging.getLogger(LOGGER_NAME)
    logger.setLevel(level)
    formatter = logging.Formatter("%(asctime)s: %(message)s", "%Y-%m-%dT%H:%M:%S%z")
    try:
        handler = logging.FileHandler(file_name, mode="a+")
        handler.setFormatter(formatter)
        logger.addHandler(handler)

    except FileNotFoundError:
        logging.warning("Could not log to file, logging to std out instead.")

    handler = logging.StreamHandler()
    handler.setFormatter(formatter)
    logger.addHandler(handler)
