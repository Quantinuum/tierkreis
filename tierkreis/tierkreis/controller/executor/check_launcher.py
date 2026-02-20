import logging
from pathlib import Path
from typing import Literal

from tierkreis.exceptions import TierkreisError

logger = logging.getLogger(__name__)


def check_and_set_launcher(
    launcher_path: Path,
    launcher_name: str,
    suffix: Literal[".sh", ".py"],
) -> Path:
    try:
        path = _exists(launcher_path, launcher_name, suffix)
        logger.warning(
            "Placing the launcher in the root directory is deprecated.\n Please move it to a 'src' subdirectory.",
        )
        return path
    except TierkreisError as e:
        try:
            return _exists(launcher_path, launcher_name, suffix, add_src=True)
        except TierkreisError as ef:
            msg = f"Launcher '{launcher_name}' not found in '{launcher_path}' or '{launcher_path}/src'."
            raise ExceptionGroup(
                msg,
                [e, ef],
            ) from ef


def _exists(
    launcher_path: Path,
    launcher_name: str,
    suffix: Literal[".sh", ".py"],
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
