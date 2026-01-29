from tierkreis.exceptions import TierkreisError


class TierkreisStorageError(TierkreisError):
    """An error with the chosen Tierkreis storage layer."""


class EntryNotFound(TierkreisStorageError):
    """Storage entry not found."""
