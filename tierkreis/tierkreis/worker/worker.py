"""Tierkreis worker implementation."""

import logging
from collections.abc import Callable
from inspect import Signature, signature
from pathlib import Path
from typing import NoReturn, TypeVar

from tierkreis.controller.data.core import PortID
from tierkreis.controller.data.location import WorkerCallArgs
from tierkreis.controller.data.models import (
    PModel,
    annotations_from_pmodel,
    dict_from_pmodel,
)
from tierkreis.controller.data.types import (
    PType,
    bytes_from_ptype,
    has_default,
    ptype_from_bytes,
)
from tierkreis.controller.storage.exceptions import EntryNotFound
from tierkreis.exceptions import TierkreisError
from tierkreis.logger_setup import add_handler_from_environment
from tierkreis.namespace import Namespace, WorkerFunction
from tierkreis.worker.storage.filestorage import WorkerFileStorage
from tierkreis.worker.storage.protocol import WorkerStorage

logger = logging.getLogger()
PrimitiveTask = Callable[[WorkerCallArgs, WorkerStorage], None]
type MethodName = str


class TierkreisWorkerError(TierkreisError):
    """Exception raised when a worker encounters an error."""


F = TypeVar("F", bound=Callable[..., PModel])


class Worker:
    """A worker bundles a set of functionality under a common namespace.

    The main usage of a worker is to convert python functions into atomic tasks,
    which can then be executed by the
    :py:class:`tierkreis.controller.executor.uv_executor.UvExecutor`
    or similar Executors.
    From the worker type stubs can be generated to statically check the function calls.

    Example snippet for wrapping a python function:

    .. code-block:: python

        worker = Worker("numpy_worker")

        @worker.function()
        def exp(x: float, a: float) -> float:
            return value = a * np.exp(x)

    :fields:
        name (str) The name of the worker.
        storage (WorkerStorage) Storage layer for the
            worker to interact with the ControllerStorage.
        namespace (Namespace) The namespace of the worker.
        types (dict[MethodName, Signature]) Mapping function names to their signatures.
        functions (dict[str, Callable[[WorkerCallArgs], None]])
            Mapping function names to their implementations.
    """

    functions: dict[str, Callable[[WorkerCallArgs], None]]
    types: dict[MethodName, Signature]
    namespace: Namespace

    def __init__(self, name: str, storage: WorkerStorage | None = None) -> None:
        self.name = name
        self.functions = {}
        self.types = {}
        self.namespace = Namespace(name=self.name, methods=[])
        if storage is None:
            self.storage: WorkerStorage = WorkerFileStorage()
        else:
            self.storage = storage

    def _load_args(
        self,
        f: WorkerFunction,
        inputs: dict[str, Path],
    ) -> dict[str, PType]:
        bs: dict[str, bytes] = {}
        for k, p in inputs.items():
            try:
                bs[k] = self.storage.read_input(p)
            except EntryNotFound as e:
                if not has_default(self.types[f.__name__].parameters[k]):
                    msg = f"Input {k} not found at {p}."
                    raise TierkreisError(msg) from e

        args = {}
        for k, b in bs.items():
            args[k] = ptype_from_bytes(
                b,
                self.types[f.__name__].parameters[k].annotation,
            )
        return args

    def _save_results(
        self,
        f: WorkerFunction,
        outputs: dict[PortID, Path],
        results: PModel,
    ) -> None:
        d = dict_from_pmodel(results)
        ret = annotations_from_pmodel(signature(f).return_annotation)
        for result_name, path in outputs.items():
            bs = bytes_from_ptype(d[result_name], ret[result_name])
            self.storage.write_output(path, bs)

    def add_types(self, func: WorkerFunction) -> None:
        """Add the types of a function to the worker.

        :param func: The function to add types for.
        :type func: WorkerFunction
        """
        self.types[func.__name__] = signature(func)

    def primitive_task(
        self,
    ) -> Callable[[PrimitiveTask], None]:
        """Register a python function as a primitive task with the worker.

        :return: The wrapped task.
        :rtype: Callable[[PrimitiveTask], None]
        """

        def function_decorator(func: PrimitiveTask) -> None:
            def wrapper(args: WorkerCallArgs) -> None:
                func(args, self.storage)

            self.functions[func.__name__] = wrapper

        return function_decorator

    def task(self) -> Callable[[F], F]:
        """Register a python function as a task with the worker.

        :return: The wrapped function.
        :rtype: Callable[[Callable[..., PModel]], Callable[..., PModel]]
        """

        def function_decorator(func: F) -> F:
            self.namespace.add_function(func)
            self.add_types(func)

            def wrapper(node_definition: WorkerCallArgs) -> None:
                kwargs = self._load_args(func, node_definition.inputs)
                results = func(**kwargs)
                self._save_results(func, node_definition.outputs, results)

            self.functions[func.__name__] = wrapper
            return func

        return function_decorator

    def run(self, worker_definition_path: Path) -> None:
        """Run a function with the parameters defined in worker_definition_path.

        :param worker_definition_path: The worker call args written by the controller.
        :type worker_definition_path: Path
        :raises TierkreisError: When the function execution results in an error.
        """
        node_definition = self.storage.read_call_args(worker_definition_path)
        logger.debug(node_definition.model_dump())

        def _check_function(msg: str) -> NoReturn:
            raise TierkreisError(msg)

        try:
            function = self.functions.get(node_definition.function_name, None)
            if function is None:
                msg = (
                    f"{self.name}: function name"
                    f"{node_definition.function_name} not found"
                )
                _check_function(msg)
            logger.info("running: %s in %s", node_definition.function_name, self.name)

            function(node_definition)

            self.storage.mark_done(node_definition.done_path)

        except Exception as err:
            logger.exception("encountered error", exc_info=err)
            self.storage.write_error(node_definition.error_path, str(err))
            msg = (
                f"Worker {self.name} encountered error when executing "
                f"{node_definition.function_name}."
            )
            raise TierkreisWorkerError(
                msg,
            ) from err

    def app(self, argv: list[str]) -> None:
        """Run the worker as uv app.

        Either generate stubs or run the worker.

        :param argv: The cli args.
        :type argv: list[str]
        """
        handler = add_handler_from_environment(logger)
        if argv[1] == "--stubs-path":
            self.namespace.write_stubs(Path(argv[2]))
        else:
            self.run(Path(argv[1]))
            logger.removeHandler(handler)
