"""Sets up the Tierkreis logger."""

import logging
import sys
from os import getenv
from pathlib import Path

from tierkreis.consts import TKR_DATE_FMT_KEY, TKR_LOG_FMT_KEY, TKR_LOG_LEVEL_KEY

LOGGER_NAME = "tierkreis"


def set_tkr_logger(
    file_name: Path,
    level: int | str = logging.INFO,
) -> None:
    """Set up the 'tierkreis' logger.

    Adds a filehandler for use in the controller.

    :param file_name: The file to use for the logging.
    :type file_name: pathlib.Path
    :param level: The log level, defaults to logging.INFO
    :type level: int | str, optional
    """
    logger = logging.getLogger(LOGGER_NAME)
    if logger.hasHandlers():
        [logger.removeHandler(h) for h in logger.handlers]
    logger.setLevel(level)
    formatter = logging.Formatter("%(asctime)s: %(message)s", "%Y-%m-%dT%H:%M:%S%z")
    try:
        handler = logging.FileHandler(file_name, mode="a+")
        handler.setFormatter(formatter)
        logger.addHandler(handler)

    except FileNotFoundError:
        root_logger = logging.getLogger()
        root_logger.warning("Could not log to file, logging to std out instead.")


def add_handler_from_environment(logger: logging.Logger) -> logging.Handler:
    """Add a handler to a logger from TKR env variables.

    Adds a stream handler on stderr with log level, format and date format
    taken from the environment variables $TKR_LOG_LEVEL, $TKR_LOG_FMT and
    $TKR_DATE_FORMAT.
    Returns the created handler so it can be removed later if needed.

    :param logger: The logger to add the handler to.
    :type logger: logging.Logger
    :return: The created handler.
    :rtype: logging.Handler
    """
    log_level = getenv(TKR_LOG_LEVEL_KEY, logging.INFO)
    if log_level is not None:
        logger.setLevel(log_level)
    log_format = getenv(TKR_LOG_FMT_KEY, None)
    date_format = getenv(TKR_DATE_FMT_KEY, None)
    formatter = logging.Formatter(log_format, date_format)
    handler = logging.StreamHandler(sys.stderr)
    handler.setFormatter(formatter)
    logger.addHandler(handler)
    return handler
