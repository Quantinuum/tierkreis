"""Namespace for a tierkreis worker."""

import shutil
import subprocess
from collections.abc import Callable
from dataclasses import dataclass, field
from inspect import Signature, signature
from logging import getLogger
from pathlib import Path
from typing import Self

from tierkreis.codegen import NO_QA_STR, format_method, format_model
from tierkreis.controller.data.models import PModel, is_portmapping
from tierkreis.controller.data.types import Struct, has_default, is_ptype
from tierkreis.exceptions import TierkreisError
from tierkreis.idl.models import GenericType, Interface, Method, Model, TypedArg
from tierkreis.idl.spec import spec

logger = getLogger(__name__)
WorkerFunction = Callable[..., PModel]


@dataclass
class Namespace:
    """The namespace of a worker.

    attr name: The name of the namespace.
    attr methods: The methods in the namespace.
    attr models: The models in the namespace.
    """

    name: str
    methods: list[Method] = field(default_factory=list)
    models: set[Model] = field(default_factory=set)

    def add_struct(self, generic_type: GenericType) -> None:
        """Add a struct to the namespace.

        :param generic_type: The generic type to add.
        :type generic_type: GenericType
        """
        if (
            not isinstance(generic_type.origin, Struct)
            or Model(is_portmapping=False, t=generic_type, decls=[]) in self.models
        ):
            return

        annotations = generic_type.origin.__annotations__
        decls = [TypedArg(k, GenericType.from_type(x)) for k, x in annotations.items()]
        for decl in decls:
            [self.add_struct(g) for g in decl.t.included_structs()]

        portmapping_flag = bool(is_portmapping(generic_type.origin))
        model = Model(portmapping_flag, generic_type, decls)
        self.models.add(model)

    @staticmethod
    def _validate_signature(func: WorkerFunction) -> Signature:
        sig = signature(func)
        for param in sig.parameters.values():
            if not is_ptype(param.annotation):
                msg = f"Expected PType got {param.annotation}"
                raise TierkreisError(msg)

        out = sig.return_annotation
        if not is_portmapping(out) and not is_ptype(out) and out is not None:
            msg = f"Expected PModel found {out}"
            raise TierkreisError(msg)

        return sig

    def add_function(self, func: WorkerFunction) -> None:
        """Add a function to the namespace.

        :param func: The function to add.
        :type func: WorkerFunction
        """
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

        for annotation_type in func.__annotations__.values():
            [
                self.add_struct(struct)
                for struct in GenericType.from_type(annotation_type).included_structs()
            ]

    @classmethod
    def from_spec_file(cls, path: Path) -> "Namespace":
        """Generate a Namespace from a tsp spec file.

        :param path: The path to the spec file.
        :type path: Path
        :return: The generated namespace.
        :rtype: Namespace
        """
        with Path.open(path) as fh:
            namespace_spec = spec(fh.read())
            return cls._from_spec(namespace_spec[0])

    @classmethod
    def _from_spec(cls, args: tuple[list[Model], Interface]) -> "Self":
        models = args[0]
        interface = args[1]
        namespace = cls(interface.name, models=set(models))
        for method in interface.methods:
            model = next((x for x in models if x.t == method.return_type), None)
            if model is not None:
                method.return_type_is_portmapping = model.is_portmapping
            namespace.methods.append(method)

        return namespace

    def stubs(self) -> str:
        """Generate type stubs strings for the namespace.

        :return: The generated stubs as string.
        :rtype: str
        """
        functions = [format_method(self.name, method) for method in self.methods]
        functions_str = "\n\n".join(functions)
        models_str = "\n\n".join([format_model(model) for model in sorted(self.models)])

        return f'''"""Code generated from {self.name} namespace. Please do not edit."""

{NO_QA_STR}
from typing import Literal, NamedTuple, Sequence, TypeVar, Generic, Protocol, Union
from types import NoneType
from tierkreis.controller.data.models import TKR, OpaqueType
from tierkreis.controller.data.types import PType, Struct, Workflow

{models_str}

{functions_str}
'''

    def write_stubs(self, stubs_path: Path) -> None:
        """Write the type stubs to stubs_path.

        :param stubs_path: The location to write to.
        :type stubs_path: Path
        """
        if not stubs_path.parent.exists():
            stubs_path.parent.mkdir(parents=True, exist_ok=True)
        with Path.open(stubs_path, "w+") as fh:
            fh.write(self.stubs())

        ruff_binary = shutil.which("ruff")
        if ruff_binary:
            subprocess.run([ruff_binary, "format", stubs_path], check=False)
            subprocess.run(
                [
                    ruff_binary,
                    "check",
                    "--fix",
                    "--ignore",
                    "D,N801,UP007",
                    "--unsafe-fixes",
                    stubs_path,
                ],
                check=False,
            )
        else:
            logger.warning("No ruff binary found. Stubs will contain raw codegen.")
