"""The storage interface."""

import json
import logging
from abc import ABC, abstractmethod
from dataclasses import asdict, dataclass
from datetime import UTC, datetime
from pathlib import Path
from typing import Any, assert_never
from uuid import UUID

from tierkreis.controller.data.core import PortID
from tierkreis.controller.data.graph import NodeDef, NodeDefModel
from tierkreis.controller.data.location import Loc, OutputLoc, WorkerCallArgs
from tierkreis.exceptions import TierkreisError

logger = logging.getLogger(__name__)


@dataclass
class StorageEntryMetadata:
    """Collection of commonly found metadata.

    Storage implementations should decide which are applicable.
    :fields:
    st_mtime (float | None): The start time of a node, defaults to None.
    """

    st_mtime: float | None = None


@dataclass
class StorageDebugData:
    """Collection of commonly found debug data.

    Currently only used for loop_nodes
    Storage implementations should decide which are applicable.
    :fields:
        loop_loc (str | None): The Loc of loop node, defaults to None.
            Can only be known after the node has run.
    """

    loop_loc: str | None = None


class ControllerStorage(ABC):
    """Storage interface for the tierkreis controller.

    :abstract:
    Conceptually, the storage layer represents the current state of the operation.
    It includes information such as:
    - Checkpoints
    - Node definitions
    - Outputs and their values
    - Metadata
    - Debug Data

    This interface primarily targets filesystems as underlying storage method.
    For storages based on other methods, you manually need to map paths
    representing node locations to an internal address.
    This interface already handles translations from Locs to Paths

    :fields:
        tkr_dir (Path): The base of the storage.
        workflow_id (UUID): The unique workflow id.
        name (str | None): Optional name for a workflow, defaults to None.

    :properties:
        workflow_dir (Path): The workflow storage location.
        logs_path (Path): Location of the workflow logs.
        debug_path (Path): Location of the workflow debug information.
    """

    tkr_dir: Path
    workflow_id: UUID
    name: str | None

    @abstractmethod
    def delete(self, path: Path) -> None:
        r"""Delete the storage entry at the specified path.

        Also delete any related data of the form \"{path}/**/*\".
        Only necessary for persistent storage types.

        :param path: The storage location to delete.
        :type path: Path
        """

    @abstractmethod
    def exists(self, path: Path) -> bool:
        """Check whether there is an entry in the storage at the specified path.

        :param path: The storage location to check.
        :type path: Path
        :return: True if it exists.
        :rtype: bool
        """

    @abstractmethod
    def link(self, src: Path, dst: Path) -> None:
        """Link storage entry at `dst` to have the same value as the entry at `src`.

        :param src: The source entry.
        :type src: Path
        :param dst: The destination entry.
        :type dst: Path
        """

    @abstractmethod
    def list_subpaths(self, path: Path) -> list[Path]:
        """List all the paths starting with the specified path in the storage.

        This is used when the number of entries can only be determined at runtime.
        For example in a map node.

        :param path: The location to list children in.
        :type path: Path
        :return: A list of child entries.
        :rtype: list[Path]
        """

    @abstractmethod
    def mkdir(self, path: Path) -> None:
        """Create an empty directory (and parents) at this storage location.

        Probably only required for file-based storage.

        :param path: The location where to create the directory.
        :type path: Path
        """

    @abstractmethod
    def read(self, path: Path) -> bytes:
        """Read the storage entry at the specified path.

        :param path: The location to read from.
        :type path: Path
        :return: The value at the storage location.
        :rtype: bytes
        """

    @abstractmethod
    def stat(self, path: Path) -> StorageEntryMetadata:
        """Get applicable stats for storage entry.

        :param path: The location to get stats for.
        :type path: Path
        :return: The stats of this entry.
        :rtype: StorageEntryMetadata
        """

    @abstractmethod
    def touch(self, path: Path) -> None:
        """Create empty storage entry at the specified location.

        :param path: The location where to generate the entry.
        :type path: Path
        """

    @abstractmethod
    def write(self, path: Path, value: bytes) -> None:
        """Write the given bytes to the storage entry at the specified path.

        :param path: The location to write to.
        :type path: Path
        :param value: The value to write.
        :type value: bytes
        """

    @property
    def workflow_dir(self) -> Path:  # noqa: D102 documented in class
        return self.tkr_dir / str(self.workflow_id)

    @property
    def logs_path(self) -> Path:  # noqa: D102 documented in class
        return self.workflow_dir / "logs"

    @property
    def debug_path(self) -> Path:  # noqa: D102 documented in class
        return self.workflow_dir / "debug"

    def _nodedef_path(self, node_location: Loc) -> Path:
        return self.workflow_dir / str(node_location) / "nodedef"

    def _worker_call_args_path(self, node_location: Loc) -> Path:
        return self.workflow_dir / str(node_location) / "definition"

    def _metadata_path(self, node_location: Loc) -> Path:
        return self.workflow_dir / str(node_location) / "_metadata"

    def _outputs_dir(self, node_location: Loc) -> Path:
        return self.workflow_dir / str(node_location) / "outputs"

    def _output_path(self, node_location: Loc, port_name: PortID) -> Path:
        return self._outputs_dir(node_location) / port_name

    def _done_path(self, node_location: Loc) -> Path:
        return self.workflow_dir / str(node_location) / "_done"

    def _error_path(self, node_location: Loc) -> Path:
        return self.workflow_dir / str(node_location) / "_error"

    def _worker_logs_path(self, node_location: Loc) -> Path:
        return self.workflow_dir / str(node_location) / "logs"

    def clean_graph_files(self) -> None:
        """Delete the workflow directory of a graph."""
        self.delete(self.workflow_dir)

    def write_node_def(self, node_location: Loc, node: NodeDef) -> None:
        """Write a node definition to storage.

        :param node_location: The location to write to.
        :type node_location: Loc
        :param node: The node definition to write.
        :type node: NodeDef
        """
        bs = NodeDefModel(root=node).model_dump_json().encode()
        self.write(self._nodedef_path(node_location), bs)

    def read_node_def(self, node_location: Loc) -> NodeDef:
        """Read the definition of a node in storage.

        :param node_location: The location to read from.
        :type node_location: Loc
        :return: The retrieved node definition.
        :rtype: NodeDef
        """
        bs = self.read(self._nodedef_path(node_location))
        return NodeDefModel(**json.loads(bs)).root

    def write_worker_call_args(
        self,
        node_location: Loc,
        function_name: str,
        inputs: dict[PortID, OutputLoc],
        output_list: list[PortID],
    ) -> Path:
        """Write the call arguments for a worker task to storage.

        :param node_location: The location to write to.
        :type node_location: Loc
        :param function_name: The task name.
        :type function_name: str
        :param inputs: The inputs to the task (outputs from previous nodes).
        :type inputs: dict[PortID, OutputLoc]
        :param output_list: The list of output ports of the task.
        :type output_list: list[PortID]
        :return: The respective path in storage.
        :rtype: Path
        """
        call_args_path = self._worker_call_args_path(node_location)
        node_definition = WorkerCallArgs(
            function_name=function_name,
            inputs={
                k: self._output_path(loc, port).relative_to(self.tkr_dir)
                for k, (loc, port) in inputs.items()
            },
            outputs={
                k: self._output_path(node_location, k).relative_to(self.tkr_dir)
                for k in output_list
            },
            output_dir=self._outputs_dir(node_location).relative_to(self.tkr_dir),
            done_path=self._done_path(node_location).relative_to(self.tkr_dir),
            error_path=self._worker_logs_path(node_location).relative_to(self.tkr_dir),
            logs_path=self.logs_path.relative_to(self.tkr_dir),
        )
        self.write(call_args_path, node_definition.model_dump_json().encode())
        self.mkdir(self._outputs_dir(node_location))

        if (parent := node_location.parent()) is not None:
            self.touch(self._metadata_path(parent))

        return call_args_path.relative_to(self.tkr_dir)

    def read_worker_call_args(self, node_location: Loc) -> WorkerCallArgs:
        """Read the worker call arguments from storage.

        :param node_location: The location to read from.
        :type node_location: Loc
        :return: The call arguments.
        :rtype: WorkerCallArgs
        """
        node_definition_path = self._worker_call_args_path(node_location)
        return WorkerCallArgs(**json.loads(self.read(node_definition_path)))

    def link_outputs(
        self,
        new_location: Loc,
        new_port: PortID,
        old_location: Loc,
        old_port: PortID,
    ) -> None:
        """Link an output from one node to another.

        Linking ensures that the values at both locations are the same.

        :param new_location: The new location to write to.
        :type new_location: Loc
        :param new_port: The port to link to.
        :type new_port: PortID
        :param old_location: The old location to read from.
        :type old_location: Loc
        :param old_port: The old location to link from.
        :type old_port: PortID
        :raises TierkreisError: If linking is not possible.
        """
        new_dir = self._output_path(new_location, new_port)
        try:
            self.link(self._output_path(old_location, old_port), new_dir)
        except FileNotFoundError as e:
            logger.warning(
                "Could not link %s to %s. Possibly a mislabelled variable?",
                e.filename,
                e.filename2,
            )
        except OSError as e:
            msg = "Workflow already exists."
            "Try running with a different ID or do_cleanup."
            raise TierkreisError(
                msg,
            ) from e

    def write_output(
        self,
        node_location: Loc,
        output_name: PortID,
        value: bytes,
    ) -> Path:
        """Write the value of an output to storage.

        :param node_location: The location to write to.
        :type node_location: Loc
        :param output_name: The port for which to write the value.
        :type output_name: PortID
        :param value: The value to write.
        :type value: bytes
        :return: The respective path in storage.
        :rtype: Path
        """
        output_path = self._output_path(node_location, output_name)
        self.write(output_path, bytes(value))
        return output_path

    def read_output(self, node_location: Loc, output_name: PortID) -> bytes:
        """Read the named output at the node location.

        :param node_location: The location to read from.
        :type node_location: Loc
        :param output_name: The port identifying the output.
        :type output_name: PortID
        :return: The value at the port.
        :rtype: bytes
        """
        return self.read(self._output_path(node_location, output_name))

    def read_errors(self, node_location: Loc) -> str:
        """Read the errors that occurred at the node location.

        Only valid for function nodes (tasks) and the top level node ("-").

        :param node_location: The location to read from.
        :type node_location: Loc
        :return: The error message that occurred. "" if nothing was logged.
        :rtype: str
        """
        if self.exists(self._worker_logs_path(node_location)):
            return self.read(self._worker_logs_path(node_location)).decode()
        if self.exists(self._error_path(node_location)):
            return self.read(self._error_path(node_location)).decode()
        return ""

    def write_node_errors(self, node_location: Loc, error_logs: str) -> None:
        """Write the errors of a node to storage.

        Only valid for function nodes (tasks) and the top level node ("-").

        :param node_location: The location to write to.
        :type node_location: Loc
        :param error_logs: The error message to write.
        :type error_logs: str
        """
        self.write(self._worker_logs_path(node_location), error_logs.encode())

    def read_output_ports(self, node_location: Loc) -> list[PortID]:
        """Read the list of named outputs of a node in storage.

        :param node_location: The location to read from.
        :type node_location: Loc
        :return: A list of output names.
        :rtype: list[PortID]
        """
        dir_list = self.list_subpaths(self._outputs_dir(node_location))
        dir_list.sort()
        return [x.name for x in dir_list]

    def is_node_started(self, node_location: Loc) -> bool:
        """Check whether a node is started.

        A node is started <==> the controller has written its definition.

        :param node_location: The location to check.
        :type node_location: Loc
        :return: True if the node is started.
        :rtype: bool
        """
        return self.exists(Path(self._nodedef_path(node_location)))

    def is_node_finished(self, node_location: Loc) -> bool:
        """Check whether a node is finished.

        A node is finished <==> A _done file/marker is set.

        :param node_location: The location to check.
        :type node_location: Loc
        :return: True if the node is finished.
        :rtype: bool
        """
        return self.exists(self._done_path(node_location))

    def latest_loop_iteration(self, loc: Loc) -> Loc:
        """Find the latest iteration location of a loop node.

        :param loc: The location to check.
        :type loc: Loc
        :return: A location representing the latest iteration.
        :rtype: Loc
        """
        i = 0
        while self.is_node_started(loc.L(i + 1)):
            i += 1
        return loc.L(i)

    def node_has_error(self, node_location: Loc) -> bool:
        """Check whether a node has encountered an error.

        Only valid for function nodes (tasks) and the top level node ("-").
        A node is errored <==> A _error file/marker is set.

        :param node_location: The location to check.
        :type node_location: Loc
        :return: True if the node has an error.
        :rtype: bool
        """
        return self.exists(self._error_path(node_location))

    def mark_node_finished(self, node_location: Loc) -> None:
        """Mark a node as successfully finished.

        :param node_location: The location to mark.
        :type node_location: Loc
        """
        self.touch(self._done_path(node_location))

        if (parent := node_location.parent()) is not None:
            self.touch(self._metadata_path(parent))

    def write_metadata(self, node_location: Loc) -> None:
        """Write the metadata for a node.

        Currently metadata contains name and start time.

        :param node_location: The location to write to.
        :type node_location: Loc
        """
        json_string = json.dumps(
            {"name": self.name, "start_time": datetime.now(UTC).isoformat()},
        )
        self.write(self._metadata_path(node_location), json_string.encode())

    def read_metadata(self, node_location: Loc) -> dict[str, Any]:
        """Read the metadata of a node.

        :param node_location: The location to read from.
        :type node_location: Loc
        :return: The metadata stored at the location.
        :rtype: dict[str, Any]
        """
        return json.loads(self.read(self._metadata_path(node_location)))

    def read_started_time(self, node_location: Loc) -> str | None:
        """Read the start time of a node.

        :param node_location: The location to read from.
        :type node_location: Loc
        :return: The time as is isoformat if the node has started.
        :rtype: str | None
        """
        node_def = Path(self._nodedef_path(node_location))
        if not self.exists(node_def):
            return None
        since_epoch = self.stat(node_def).st_mtime
        if since_epoch is None:
            return None
        return datetime.fromtimestamp(since_epoch, UTC).isoformat()

    def read_finished_time(self, node_location: Loc) -> str | None:
        """Read the finish time of a node.

        :param node_location: The location to read from.
        :type node_location: Loc
        :return: The time as is isoformat if the node has finished.
        :rtype: str | None
        """
        done = Path(self._done_path(node_location))
        if not self.exists(done):
            return None
        since_epoch = self.stat(done).st_mtime
        if since_epoch is None:
            return None
        return datetime.fromtimestamp(since_epoch, UTC).isoformat()

    def read_loop_trace(self, node_location: Loc, output_name: PortID) -> list[bytes]:
        """Read the trace of a loop node for a given output.

        A trace is a list of values v, where v[i] represents the value of the output at
        iteration i.

        :param node_location: The location to read from.
        :type node_location: Loc
        :param output_name: The output to trace.
        :type output_name: PortID
        :raises TierkreisError: If the node at the location is not a loop node.
        :return: A list of values considered the trace.
        :rtype: list[bytes]
        """
        definition = self.read_node_def(node_location)
        if definition.type != "loop":
            msg = "Can only read traces from loop nodes."
            raise TierkreisError(msg)
        result = []

        i = 0
        while self.is_node_started(node_location.L(i)):
            result.append(self.read_output(node_location.L(i), output_name))
            i += 1
        return result

    def loc_from_node_name(self, node_name: str) -> Loc | None:
        """Find the location of a node for a given name.

        Currently only loop nodes can be named.
        Loop names are stored as debug data.
        This can only be invoked after running the workflow.

        :param node_name: The name to search for.
        :type node_name: str
        :return: Returns the location if found in storage.
        :rtype: Loc | None
        """
        debug_data = StorageDebugData(**self.read_debug_data(node_name))
        if debug_data.loop_loc is not None:
            return Loc(debug_data.loop_loc)
        return None

    def write_debug_data(self, name: str, node_location: Loc) -> None:
        """Write the debug data of a node.

        Currently name is derived from a named loop.

        :param name: The name to write to.
        :type name: str
        :param node_location: The location to write as debug information.
        :type node_location: Loc
        """
        self.mkdir(self.debug_path)
        data = StorageDebugData(loop_loc=node_location)
        self.write(self.debug_path / name, json.dumps(asdict(data)).encode())

    def read_debug_data(self, name: str) -> dict[str, Any]:
        """Read the debug data for a given name.

        Currently name is derived from a named loop.

        :param name: The name to read from.
        :type name: str
        :return: The data available for the name.
        :rtype: dict[str, Any]
        """
        return json.loads(self.read(self.debug_path / name))

    def dependents(self, node_location: Loc) -> set[Loc]:
        """Get the dependents (successors) of a node.

        Dependents are nodes that are fully invalidated if
        the node at the given loc is invalidated.
        This does not include the direct parent Loc,
        which is only partially invalidated.

        :param loc: The location to get the dependents for.
        :type loc: Loc
        :return: A set of dependent nodes.
        :rtype: set[Loc]
        """
        descendants: set[Loc] = set()
        step, parent = node_location.pop_last()
        match step:
            case "-":
                pass
            case ("N", _):
                nodedef = self.read_node_def(node_location)
                if nodedef.type == "output":
                    descendants.update(self.dependents(parent))
                for output in nodedef.outputs.values():
                    descendants.add(parent.N(output))
                    descendants.update(self.dependents(parent.N(output)))
            case ("M", _):
                descendants.update(self.dependents(parent))
            case ("L", idx):
                latest_idx = self.latest_loop_iteration(parent).peek_index()
                [descendants.add(parent.L(i)) for i in range(idx + 1, latest_idx + 1)]
                descendants.update(self.dependents(parent))
            case _:
                assert_never(step)

        return descendants

    def restart_task(self, loc: Loc) -> list[Loc]:
        """Restart the task/function node at the given loc.

        A node is restarted by removing its (and its dependents) started flag.
        The controller will then pick it up as not started.
        Fully dependent nodes will be completely removed from the storage.
        The parent locs will be partially invalidated.

        Returns the invalidated nodes.

        :param loc: _description_
        :type loc: Loc
        :raises TierkreisError: If the node at the location is not a task.
        :return: _description_
        :rtype: list[Loc]
        """
        nodedef = self.read_node_def(loc)
        if nodedef.type != "function":
            msg = "Can only restart task/function nodes."
            raise TierkreisError(msg)

        # Remove fully invalidated nodes.
        deps = self.dependents(loc)
        [self.delete(self.workflow_dir / a) for a in deps]

        # Mark partially invalidated nodes as not finished and remove their outputs.
        partials = loc.partial_locs()
        [self.delete(self._done_path(x)) for x in partials]
        [self.delete(self.workflow_dir / a / "outputs") for a in partials]

        # Mark given Loc as not started
        # so that the controller picks it up on the next tick.
        self.delete(self._nodedef_path(loc))

        return list(deps)
