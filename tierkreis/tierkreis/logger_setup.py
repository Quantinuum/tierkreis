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


def update_logger_from_environment(logger: logging.Logger, handler_index: int) -> None:
    if not logger.hasHandlers():
        logger.error("Logger has no handlers to update.")
        return
    if not (0 <= handler_index < len(logger.handlers)):
        return
    log_level = getenv(TKR_LOG_LEVEL_KEY, None)
    if log_level is not None:
        logger.setLevel(log_level)
    formatter = logger.handlers[handler_index].formatter
    log_format = getenv(TKR_LOG_FMT_KEY, None)
    date_format = getenv(TKR_DATE_FMT_KEY, None)

    if formatter is None:
        formatter = logging.Formatter(log_format, date_format)
        logger.handlers[handler_index].setFormatter(formatter)
        return
    new_formatter = logging.Formatter(
        log_format or formatter._fmt, date_format or formatter.datefmt
    )
    logger.handlers[handler_index].setFormatter(new_formatter)
