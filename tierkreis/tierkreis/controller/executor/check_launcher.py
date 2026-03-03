"""Utilities to find the correct executable for a worker."""

import logging
import shutil
from pathlib import Path
from typing import Literal

from tierkreis.exceptions import TierkreisError

logger = logging.getLogger(__name__)


def check_and_set_launcher(
    launcher_path: Path | None,
    launcher_name: str,
    suffix: Literal[".sh", ".py"],
    *,
    check_shell: bool = False,
) -> Path:
    """Find the correct executable for a worker.

    Given the directory and a worker name searches for
    1. main.py (.sh)
    2. src/main.py (.sh)

    If `check_shell` is set or `launcher_path` is None
    will check if one of the following is in $PATH (using which):
    1. launcher_name
    2. tkr_launcher_name_impl
    3. tkr-launcher-name-impl

    :param launcher_path: The directory to search.
    :type launcher_path: Path | None
    :param launcher_name: The name of the worker to find.
    :type launcher_name: str
    :param suffix: External or internal worker (.py or .sh).
    :type suffix: Literal['.sh', '.py']
    :param check_shell: To extend the search to check $PATH, defaults to false.
    :type check_shell: bool
    :raises TierkreisError: If neither of the expected paths exist.
    :return: The full path to the worker executable.
    :rtype: Path
    """
    if check_shell or launcher_path is None:
        if (path := shutil.which(launcher_name)) is not None:
            return Path(path)
        if (path := _check_tkr_worker_command(launcher_name)) is not None:
            return Path(path)
    if launcher_path is None:
        msg = f"{launcher_name} is not a shell command, but no launcher_path was provided."
        raise TierkreisError(msg)
    try:
        path = _exists(launcher_path, launcher_name, suffix)
    except TierkreisError as e:
        try:
            return _exists(launcher_path, launcher_name, suffix, add_src=True)
        except TierkreisError as ef:
            msg = (
                f"Launcher '{launcher_name}' not found in"
                f" '{launcher_path}' or '{launcher_path}/src'."
            )
            raise ExceptionGroup(
                msg,
                [e, ef],
            ) from ef
    else:
        logger.warning(
            "Placing the launcher in the root directory is deprecated.\n"
            "Please move it to a 'src' subdirectory.",
        )
        return path


def _check_tkr_worker_command(launcher_name: str) -> str | None:
    cmd = f"tkr-{launcher_name}-impl".replace("_", "-")
    path = shutil.which(cmd)
    if path is not None:
        return path
    cmd = f"tkr-{launcher_name}-impl".replace("-", "_")
    path = shutil.which(cmd)
    if path is not None:
        return path
    return None


def _exists(
    launcher_path: Path,
    launcher_name: str,
    suffix: Literal[".sh", ".py"],
    *,
    add_src: bool = False,
) -> Path:
    launcher_path = launcher_path / launcher_name
    if add_src:
        launcher_path = launcher_path / "src"
    if not launcher_path.exists():
        msg = f"Launcher not found: {launcher_name}."
        raise TierkreisError(msg)

    if launcher_path.is_dir() and not (launcher_path / f"main{suffix}").exists():
        msg = f"Expected launcher file. Got {launcher_path}."
        raise TierkreisError(msg)

    if launcher_path.is_dir() and not (launcher_path / f"main{suffix}").is_file():
        msg = f"Expected launcher file. Got {launcher_path}/main{suffix}"
        raise TierkreisError(
            msg,
        )
    if launcher_path.is_dir() and (launcher_path / f"main{suffix}").is_file():
        launcher_path = launcher_path / f"main{suffix}"
    return launcher_path
