from logging import getLogger
from pathlib import Path
from typing import Any, Literal, Optional

from pydantic import BaseModel, GetCoreSchemaHandler
from pydantic_core import CoreSchema, core_schema
from tierkreis.controller.data.core import PortID
from typing_extensions import assert_never

from tierkreis.controller.data.core import NodeIndex
from tierkreis.exceptions import TierkreisError

logger = getLogger(__name__)


class WorkerCallArgs(BaseModel):
    function_name: str
    inputs: dict[str, Path]
    outputs: dict[str, Path]
    output_dir: Path
    done_path: Path
    error_path: Path
    logs_path: Optional[Path]


NodeStep = Literal["-"] | tuple[Literal["N", "L", "M"], NodeIndex]


class Loc(str):
    def __new__(cls, k: str = "-") -> "Loc":
        return super(Loc, cls).__new__(cls, k)

    def N(self, idx: int) -> "Loc":
        return Loc(str(self) + f".N{idx}")

    def L(self, idx: int) -> "Loc":
        return Loc(str(self) + f".L{idx}")

    def M(self, idx: int) -> "Loc":
        return Loc(str(self) + f".M{idx}")

    @staticmethod
    def from_steps(steps: list[NodeStep]) -> "Loc":
        loc = ""
        for step in steps.copy():
            match step:
                case "-":
                    loc += "-"
                case (node_type, idx):
                    loc += f".{node_type}{idx}"
        return Loc(loc)

    def parent(self) -> "Loc | None":
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
                    raise TierkreisError(f"Invalid Loc: {self}")

        return steps

    @classmethod
    def __get_pydantic_core_schema__(
        cls, source_type: Any, handler: GetCoreSchemaHandler
    ) -> CoreSchema:
        return core_schema.no_info_after_validator_function(cls, handler(str))

    def pop_first(self) -> tuple[NodeStep, "Loc"]:
        if self == "-":
            return "-", Loc("")
        steps = self.steps()
        if len(steps) < 2:
            raise TierkreisError("Malformed Loc")
        first = steps.pop(1)
        if first == "-":
            raise TierkreisError("Malformed Loc")
        return first, Loc.from_steps(steps)

    def pop_last(self) -> tuple[NodeStep, "Loc"]:
        if self == "-":
            return "-", Loc("")
        steps = self.steps()
        if len(steps) < 2:
            raise TierkreisError("Malformed Loc")
        last = steps.pop(-1)
        if last == "-":
            raise TierkreisError("Malformed Loc")
        return last, Loc.from_steps(steps)


def get_last_index(loc: Loc) -> int:
    step = loc.steps()[-1]
    if isinstance(step, str):
        return 0
    return step[1]


OutputLoc = tuple[Loc, PortID]
