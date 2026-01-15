"""Storage protocol for workers."""

from pathlib import Path
from typing import Protocol

from tierkreis.controller.data.location import WorkerCallArgs


class WorkerStorage(Protocol):
    """Storage protocol for workers.

    :abstract:
    """

    def resolve(self, path: Path | str) -> Path:
        """Resolve a path or str to a path in the storage.

        Transforms paths such that the storage can correctly read/write to them.
        E.g. for a file storage, relative paths are resolved against a base directory.

        :param path: The path to resolve.
        :type path: Path | str
        :return: The resolved path according to the storage.
        :rtype: Path
        """
        ...

    def read_call_args(self, path: Path) -> WorkerCallArgs:
        """Read the call args of a worker.

        The function name is part of the call args which is then used to
        determine which function to call.

        :param path: The path to read from.
        :type path: Path
        :return: The call args of the worker function.
        :rtype: WorkerCallArgs
        """
        ...

    def read_input(self, path: Path) -> bytes:
        """Read the input to a worker task.

        Input locations are defined in the call args.

        :param path: The path to read from.
        :type path: Path
        :return: The bytes read from the input location.
        :rtype: bytes
        """
        ...

    def write_output(self, path: Path, value: bytes) -> None:
        """Write the outputs of a worker task.

        Output locations are defined in the call args.

        :param path: The path to write to.
        :type path: Path
        :param value: The bytes to write.
        :type value: bytes
        """
        ...

    def glob(self, path_string: str) -> list[str]:
        """Get a list of paths matching the path glob.

        Used in map nodes to find all input values for the individual tasks.

        :param path_string: The glob string to match.
        :type path_string: str
        :return: A list of matching path strings.
        :rtype: list[str]
        """
        ...

    def mark_done(self, path: Path) -> None:
        """Mark the task node as done.

        Done paths are defined in the call args.

        :param path: The path to mark as done.
        :type path: Path
        """
        ...

    def write_error(self, path: Path, error_logs: str) -> None:
        """Write an error to the logs.

        Logs are stored in a location defined in the call args.

        :param path: The path to write the error logs to.
        :type path: Path
        :param error_logs: The message to write.
        :type error_logs: str
        """
        ...
