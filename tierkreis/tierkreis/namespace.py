from dataclasses import dataclass, field
from inspect import Signature, signature
from logging import getLogger
from pathlib import Path
import shutil
import subprocess
from typing import Callable, Self
from tierkreis.codegen import format_method, format_model
from tierkreis.controller.data.models import PModel, is_portmapping
from tierkreis.controller.data.types import Struct, has_default, is_ptype
from tierkreis.exceptions import TierkreisError
from tierkreis.idl.spec import spec
from tierkreis.idl.models import GenericType, Interface, Method, Model, TypedArg


logger = getLogger(__name__)
WorkerFunction = Callable[..., PModel]


@dataclass
class Namespace:
    name: str
    methods: list[Method] = field(default_factory=lambda: [])
    models: set[Model] = field(default_factory=lambda: set())

    def add_struct(self, gt: GenericType) -> None:
        if not isinstance(gt.origin, Struct) or Model(False, gt, []) in self.models:
            return

        annotations = gt.origin.__annotations__
        decls = [TypedArg(k, GenericType.from_type(x)) for k, x in annotations.items()]
        for decl in decls:
            [self.add_struct(g) for g in decl.t.included_structs()]

        portmapping_flag = True if is_portmapping(gt.origin) else False
        model = Model(portmapping_flag, gt, decls)
        self.models.add(model)

    @staticmethod
    def _validate_signature(func: WorkerFunction) -> Signature:
        sig = signature(func)
        for param in sig.parameters.values():
            if not is_ptype(param.annotation):
                raise TierkreisError(f"Expected PType got {param.annotation}")

        out = sig.return_annotation
        if not is_portmapping(out) and not is_ptype(out) and out is not None:
            raise TierkreisError(f"Expected PModel found {out}")

        return sig

    def add_function(self, func: WorkerFunction) -> None:
        sig = self._validate_signature(func)

        method = Method(
            GenericType(func.__name__, [str(x) for x in func.__type_params__]),
            [
                TypedArg(k, GenericType.from_type(t.annotation), has_default(t))
                for k, t in sig.parameters.items()
            ],
            GenericType.from_type(sig.return_annotation),
            is_portmapping(sig.return_annotation),
        )
        self.methods.append(method)

        for t in func.__annotations__.values():
            [self.add_struct(x) for x in GenericType.from_type(t).included_structs()]

    @classmethod
    def from_spec_file(cls, path: Path) -> "Namespace":
        with open(path) as fh:
            namespace_spec = spec(fh.read())
            return cls._from_spec(namespace_spec[0])

    @classmethod
    def _from_spec(cls, args: tuple[list[Model], Interface]) -> "Self":
        models = args[0]
        interface = args[1]
        namespace = cls(interface.name, models=set(models))
        for f in interface.methods:
            model = next((x for x in models if x.t == f.return_type), None)
            if model is not None:
                f.return_type_is_portmapping = model.is_portmapping
            namespace.methods.append(f)

        return namespace

    def stubs(self) -> str:
        functions = [format_method(self.name, f) for f in self.methods]
        functions_str = "\n\n".join(functions)
        models_str = "\n\n".join([format_model(x) for x in sorted(list(self.models))])

        return f'''"""Code generated from {self.name} namespace. Please do not edit."""

from typing import Literal, NamedTuple, Sequence, TypeVar, Generic, Protocol, Union
from types import NoneType
from tierkreis.controller.data.models import TKR, OpaqueType
from tierkreis.controller.data.types import PType, Struct

{models_str}

{functions_str}
'''

    def write_stubs(self, stubs_path: Path) -> None:
        """Writes the type stubs to stubs_path.

        :param stubs_path: The location to write to.
        :type stubs_path: Path
        """
        with open(stubs_path, "w+") as fh:
            fh.write(self.stubs())

        ruff_binary = shutil.which("ruff")
        if ruff_binary:
            subprocess.run([ruff_binary, "format", stubs_path])
            subprocess.run([ruff_binary, "check", "--fix", stubs_path])
        else:
            logger.warning("No ruff binary found. Stubs will contain raw codegen.")
