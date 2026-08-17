"""Data structures for node locations in the controller."""

from logging import getLogger
from pathlib import Path
from typing import Any, Literal, Self, assert_never

from pydantic import BaseModel, GetCoreSchemaHandler
from pydantic_core import CoreSchema, core_schema

from tierkreis.controller.data.core import NodeIndex, PortID
from tierkreis.exceptions import TierkreisError

logger = getLogger(__name__)


class WorkerCallArgs(BaseModel):
    """The arguments for a worker call.

    Contains all information.
    This will be provided as first

    :fields:
        function_name (str): The name of the function to call.
        inputs (dict[str, Path]): The input paths for the worker.
            Each input is provided as a path to a file containing the input data.
        outputs (dict[str, Path]): The output paths for the worker.
            Each output is provided as a path to a file where the worker
             should write the output data.
        output_dir (Path): The directory for the worker outputs.
        done_path (Path): The path to touch to indicate finished successfully.
        error_path (Path): The path to touch to indicate finished with error.
        logs_path (Path | None): The path to write logs to.
    """

    function_name: str
    inputs: dict[str, Path]
    outputs: dict[str, Path]
    output_dir: Path
    done_path: Path
    error_path: Path
    logs_path: Path | None


NodeStep = Literal["-"] | tuple[Literal["N", "L", "M"], NodeIndex]

MIN_LENGTH = 2


class Loc(str):
    """The Loc(ation) of a node in storage.

    This is a string that encodes the path to the node in the graph.
    """

    __slots__ = []

    def __new__(cls, k: str = "-") -> Self:
        """Construct a new Loc.

        :param k: The location string., defaults to "-"
        :type k: str, optional
        :return: The representation of self.
        :rtype: Self
        """
        return super().__new__(cls, k)

    def N(self, idx: int) -> "Loc":
        """Append a regular node.

        Regular nodes are all but loops and maps.

        :param idx: The index of the node to append.
        :type idx: int
        :return: The new location.
        :rtype: Loc
        """
        return Loc(str(self) + f".N{idx}")

    def L(self, idx: int) -> "Loc":
        """Append a loop node.

        L<number> is one iteration of a loop.

        :param idx: The index of the loop iteration to append.
        :type idx: int
        :return: The new Location.
        :rtype: Loc
        """
        return Loc(str(self) + f".L{idx}")

    def M(self, idx: int) -> "Loc":
        """Append a map node.

        M<number> is one element of a map.

        :param idx: The index of the map iteration to append.
        :type idx: int
        :return: The new location.
        :rtype: Loc
        """
        return Loc(str(self) + f".M{idx}")

    @staticmethod
    def from_steps(steps: list[NodeStep]) -> "Loc":
        """Construct a location from steps.

        Steps are separated by '.' and consist of
        a node type and index.

        :param steps: The list of steps to construct the location from.
        :type steps: list[NodeStep]
        :return: The new location.
        :rtype: Loc
        """
        loc = ""
        for step in steps.copy():
            match step:
                case "-":
                    loc += "-"
                case (node_type, idx):
                    loc += f".{node_type}{idx}"
        return Loc(loc)

    def parent(self) -> "Loc | None":
        """Return the parent of the node.

        :return: The parent location, or none if it is a root.
        :rtype: Loc | None
        """
        steps = self.steps()
        if not steps:
            return None

        last_step = steps.pop()
        match last_step:
            case "-":
                return Loc.from_steps([])
            case ("L", 0):
                return Loc.from_steps(steps)
            case ("L", idx):
                return Loc.from_steps(steps).L(idx - 1)
            case ("N", idx) | ("M", idx):
                return Loc.from_steps(steps)
            case _:
                assert_never(last_step)

    def steps(self) -> list[NodeStep]:
        """Deconstruct self into steps.

        Steps are separated by '.' and consist of
        a node type and index.

        :raises TierkreisError: On a malformed location string.
        :return: The list of steps.
        :rtype: list[NodeStep]
        """
        if self == "":
            return []

        steps: list[NodeStep] = []
        for step_str in self.split("."):
            match step_str[0], step_str[1:]:
                case ("-", _):
                    steps.append("-")
                case ("N", idx_str):
                    steps.append(("N", int(idx_str)))
                case ("L", idx_str):
                    steps.append(("L", int(idx_str)))
                case ("M", idx_str):
                    steps.append(("M", int(idx_str)))
                case _:
                    msg = f"Invalid Loc: {self}"
                    raise TierkreisError(msg)

        return steps

    @classmethod
    def __get_pydantic_core_schema__(
        cls,
        source_type: Any,
        handler: GetCoreSchemaHandler,
    ) -> CoreSchema:
        """Make Loc work with pydantic."""
        return core_schema.no_info_after_validator_function(cls, handler(str))

    def pop_first(self) -> tuple[NodeStep, "Loc"]:
        """Pop the first step of the location.

        The remaining location is still valid.

        e.g. for Loc("N0.L1.N3"), this returns ("N", 0) and Loc("L1.N3").

        :raises TierkreisError: On a malformed location string.
        :return: The first step and the remaining location.
        :rtype: tuple[NodeStep, Loc]
        """
        if self == "-":
            return "-", Loc("")
        steps = self.steps()
        if len(steps) < MIN_LENGTH:
            msg = "Malformed Loc"
            raise TierkreisError(msg)
        first = steps.pop(1)
        if first == "-":
            msg = "Malformed Loc"
            raise TierkreisError(msg)
        return first, Loc.from_steps(steps)

    def pop_last(self) -> tuple[NodeStep, "Loc"]:
        """Pop the last step of the location.

        The remaining location is still valid.

        e.g. for Loc("N0.L1.N3"), this returns ("N", 3) and Loc("N0.L1").

        :raises TierkreisError: On a malformed location string.
        :return: The last step and the remaining location.
        :rtype: tuple[NodeStep, Loc]
        """
        if self == "-":
            return "-", Loc("")
        steps = self.steps()
        if len(steps) < MIN_LENGTH:
            msg = "Malformed Loc"
            raise TierkreisError(msg)
        last = steps.pop(-1)
        if last == "-":
            msg = "Malformed Loc"
            raise TierkreisError(msg)
        return last, Loc.from_steps(steps)

    def peek(self) -> NodeStep:
        """Get the last step without removing.

        :return: The last step of the loc.
        :rtype: NodeStep
        """
        return self.steps()[-1]

    def peek_index(self) -> int:
        """Get the index of the last step.

        :return: The index of the last node in the locaction.
        :rtype: int
        """
        step = self.steps()[-1]

        if isinstance(step, str):
            return 0
        return step[1]

    def partial_locs(self) -> list["Loc"]:
        """Generate all partial locations contained in self.

        :return: A list of all partial locations contained in self
            including self and root.
        :rtype: list[Loc]
        """
        steps = self.steps()
        return [Loc.from_steps(steps[: i + 1]) for i in range(len(steps))]


OutputLoc = tuple[Loc, PortID]
