"""Tierkreis exception definitions."""

from pathlib import Path


class TierkreisError(Exception):
    """An error thrown in the Tierkreis library."""


class TierkreisStorageError(TierkreisError):
    """An error with the chosen Tierkreis storage layer."""


class EntryNotFoundError(TierkreisStorageError):
    """Storage entry not found."""

    path: Path

    def __init__(self, path: Path) -> None:
        self.path = path

    def __str__(self) -> str:
        """Print string of self."""
        return str(self.path)
