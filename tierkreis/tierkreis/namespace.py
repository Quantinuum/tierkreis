"""Namespace for a tierkreis worker."""

import shutil
import subprocess
from collections.abc import Callable
from dataclasses import dataclass, field
from inspect import Signature, signature, isclass
from logging import getLogger
from pathlib import Path
from typing import Self, get_args, get_origin, Annotated

from tierkreis.codegen import format_method, format_model
from tierkreis.controller.data.core import RestrictedNamedTuple
from tierkreis.controller.data.models import PModel, TKR, is_portmapping
from tierkreis.controller.data.types import FinishedGraph, has_default, is_ptype
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

    def add_struct(self, ty: type) -> None:
        """Add a struct to the namespace.

        :param generic_type: The generic type to add.
        :type generic_type: GenericType
        """
        if (
            Model(is_portmapping=False, t=GenericType.from_type(ty), decls=[])
            in self.models
        ):
            return
        for t in all_structs(ty):
            portmapping_flag = bool(is_portmapping(t.origin))
            annotations = t.origin.__annotations__
            decls = [
                TypedArg(k, GenericType.from_type(x)) for k, x in annotations.items()
            ]
            if portmapping_flag:
                is_t = False
            else:
                are_ptypes = set(is_ptype(x) for x in annotations.values())
                if len(are_ptypes) != 1:
                    raise TypeError(
                        f"Struct {t} has fields that are not all PTypes or all non-PTypes."
                    )
                is_t = not next(iter(are_ptypes))
            model = Model(portmapping_flag, t, decls, is_t)
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
            self.add_struct(annotation_type)

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

from typing import Literal, NamedTuple, Sequence, TypeVar, Generic, Protocol, Union
from types import NoneType
from tierkreis.controller.data.models import TKR, OpaqueType
from tierkreis.controller.data.types import PType, Struct, FinishedGraph

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


# ALAN This takes `type` rather than `GenericType` because is_ptype requires a `type`.
def all_structs(ty: type) -> set[GenericType]:
    """Find all included structs of this type, including its fields.

    :return: A set of types
    :rtype: set[GenericType]
    """

    outs = set()

    def scan(ty: type):
        g = GenericType.from_type(ty)
        if g in outs:
            return
        if get_origin(ty) is Annotated:
            ty = get_args(ty)[0]
        if (
            isclass(g.origin) and issubclass(g.origin, RestrictedNamedTuple)
        ) or is_portmapping(g.origin):
            outs.add(g)
            for field_ty in g.origin.__annotations__.values():
                scan(field_ty)
            # fallthrough to scan type-args
        elif isinstance(ty, str) or (g.origin is TKR):
            # Occurrences of TypeVars within the body (field defns) of a generic class
            # seem to be returned as GenericAlias's rather than TypeVar's.
            return  # skip
        elif not is_ptype(ty) and g.origin is not FinishedGraph:
            raise TypeError(f"Unexpected type {ty} in all_structs")
        # ptype's may contain nested structs of interest, but are not interesting themselves.

        for x in get_args(ty):
            scan(x)

    scan(ty)
    return outs
