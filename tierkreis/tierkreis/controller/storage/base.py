from abc import ABC, abstractmethod
from dataclasses import dataclass
from datetime import datetime
import json
from pathlib import Path
from typing import Any
from uuid import UUID
from tierkreis.controller.data.graph import NodeDef, NodeDefModel
from tierkreis.controller.data.location import Loc, OutputLoc, WorkerCallArgs
from tierkreis.controller.data.core import PortID
from tierkreis.exceptions import TierkreisError


@dataclass
class StatResult:
    st_mtime: float


class TKRStorage(ABC):
    tkr_dir: Path
    workflow_id: UUID
    name: str | None

    @abstractmethod
    def delete(self) -> None: ...

    @abstractmethod
    def exists(self, path: Path) -> bool: ...

    @abstractmethod
    def list_output_paths(self, output_dir: Path) -> list[Path]: ...

    @abstractmethod
    def link(self, src: Path, dst: Path) -> None: ...

    @abstractmethod
    def read(self, path: Path) -> bytes: ...

    @abstractmethod
    def stat(self, path: Path) -> StatResult: ...

    @abstractmethod
    def touch(self, path: Path, is_dir: bool = False) -> None: ...

    @abstractmethod
    def write(self, path: Path, value: bytes) -> None: ...

    @property
    def workflow_dir(self) -> Path:
        return self.tkr_dir / str(self.workflow_id)

    @property
    def logs_path(self) -> Path:
        return self.workflow_dir / "logs"

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

    def _error_logs_path(self, node_location: Loc) -> Path:
        return self.workflow_dir / str(node_location) / "errors"

    def clean_graph_files(self) -> None:
        self.delete()

    def write_node_def(self, node_location: Loc, node: NodeDef):
        bs = NodeDefModel(root=node).model_dump_json().encode()
        self.write(self._nodedef_path(node_location), bs)

    def read_node_def(self, node_location: Loc) -> NodeDef:
        bs = self.read(self._nodedef_path(node_location))
        return NodeDefModel(**json.loads(bs)).root

    def write_worker_call_args(
        self,
        node_location: Loc,
        function_name: str,
        inputs: dict[PortID, OutputLoc],
        output_list: list[PortID],
    ) -> Path:
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
            error_path=self._error_path(node_location).relative_to(self.tkr_dir),
            logs_path=self.logs_path.relative_to(self.tkr_dir),
        )
        self.write(call_args_path, node_definition.model_dump_json().encode())
        self.touch(self._outputs_dir(node_location), is_dir=True)

        if (parent := node_location.parent()) is not None:
            self.touch(self._metadata_path(parent))

        return call_args_path.relative_to(self.tkr_dir)

    def read_worker_call_args(self, node_location: Loc) -> WorkerCallArgs:
        node_definition_path = self._worker_call_args_path(node_location)
        return WorkerCallArgs(**json.loads(self.read(node_definition_path)))

    def link_outputs(
        self,
        new_location: Loc,
        new_port: PortID,
        old_location: Loc,
        old_port: PortID,
    ) -> None:
        new_dir = self._output_path(new_location, new_port)
        self.touch(self._outputs_dir(new_location), is_dir=True)
        try:
            self.link(self._output_path(old_location, old_port), new_dir)
        except FileNotFoundError as e:
            raise TierkreisError(
                f"Could not link {e.filename} to {e.filename2}."
                " Possibly a mislabelled variable?"
            )
        except OSError as e:
            raise TierkreisError(
                "Workflow already exists. Try running with a different ID or do_cleanup."
            ) from e

    def write_output(
        self, node_location: Loc, output_name: PortID, value: bytes
    ) -> Path:
        output_path = self._output_path(node_location, output_name)
        self.write(output_path, bytes(value))
        return output_path

    def read_output(self, node_location: Loc, output_name: PortID) -> bytes:
        return self.read(self._output_path(node_location, output_name))

    def read_errors(self, node_location: Loc) -> str:
        if not self.exists(self._error_logs_path(node_location)):
            if self.exists(self._error_path(node_location)):
                return self.read(self._error_path(node_location)).decode()
            return ""
        errors = self.read(self._error_logs_path(node_location)).decode()
        if errors == "":
            if self.exists(self._error_path(node_location)):
                return self.read(self._error_path(node_location)).decode()
        return errors

    def write_node_errors(self, node_location: Loc, error_logs: str) -> None:
        self.write(self._error_logs_path(node_location), error_logs.encode())

    def read_output_ports(self, node_location: Loc) -> list[PortID]:
        dir_list = self.list_output_paths(self._outputs_dir(node_location))
        dir_list.sort()
        return [x.name for x in dir_list]

    def is_node_started(self, node_location: Loc) -> bool:
        return self.exists(Path(self._nodedef_path(node_location)))

    def is_node_finished(self, node_location: Loc) -> bool:
        return self.exists(self._done_path(node_location))

    def node_has_error(self, node_location: Loc) -> bool:
        return self.exists(self._error_path(node_location))

    def mark_node_finished(self, node_location: Loc) -> None:
        self.touch(self._done_path(node_location))

        if (parent := node_location.parent()) is not None:
            self.touch(self._metadata_path(parent))

    def write_metadata(self, node_location: Loc) -> None:
        j = json.dumps({"name": self.name, "start_time": datetime.now().isoformat()})
        self.write(self._metadata_path(node_location), j.encode())

    def read_metadata(self, node_location: Loc) -> dict[str, Any]:
        return json.loads(self.read(self._metadata_path(node_location)))

    def read_started_time(self, node_location: Loc) -> str | None:
        node_def = Path(self._nodedef_path(node_location))
        if not self.exists(node_def):
            return None
        since_epoch = self.stat(node_def).st_mtime
        return datetime.fromtimestamp(since_epoch).isoformat()

    def read_finished_time(self, node_location: Loc) -> str | None:
        done = Path(self._done_path(node_location))
        if not self.exists(done):
            return None
        since_epoch = self.stat(done).st_mtime
        return datetime.fromtimestamp(since_epoch).isoformat()
