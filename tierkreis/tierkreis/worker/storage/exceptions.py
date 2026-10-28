"""Errors raised by worker asset storage implementations."""

from pathlib import Path

from tierkreis.exceptions import TierkreisError


class WorkerStorageError(TierkreisError):
    """An error raised by a worker storage layer."""


class EntryNotFoundError(WorkerStorageError):
    """A requested worker storage entry was not found."""

    def __init__(self, path: Path) -> None:
        self.path = path

    def __str__(self) -> str:
        """Return the missing path."""
        return str(self.path)
