import logging
from os import getenv
from pathlib import Path

from tierkreis.consts import TKR_DATE_FMT_KEY, TKR_LOG_FMT_KEY, TKR_LOG_LEVEL_KEY

LOGGER_NAME = "TKR"


def set_tkr_logger(
    file_name: Path,
    level: int | str = logging.INFO,
) -> None:
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


def add_handler_from_environment(logger: logging.Logger) -> None:
    log_level = getenv(TKR_LOG_LEVEL_KEY, None)
    if log_level is not None:
        logger.setLevel(log_level)
    log_format = getenv(TKR_LOG_FMT_KEY, None)
    date_format = getenv(TKR_DATE_FMT_KEY, None)
    formatter = logging.Formatter(log_format, date_format)
    handler = logging.StreamHandler()
    handler.setFormatter(formatter)
    logger.addHandler(handler)
