from pathlib import Path

from tierkreis.exceptions import TierkreisError


class TierkreisStorageError(TierkreisError):
    """An error with the chosen Tierkreis storage layer."""


class EntryNotFound(TierkreisStorageError):
    """Storage entry not found."""

    path: Path

    def __init__(self, path: Path) -> None:
        self.path = path

    def __str__(self) -> str:
        return str(self.path)
