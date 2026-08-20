from typing import NamedTuple

from tierkreis.models import portmapping

from tierkreis import Worker

worker = Worker("multiple_outputs_worker")


class OpaquePoint(NamedTuple):
    x: float
    y: float
    z: float


@worker.task()
def new_opaque_point(x: float, y: float, z: float) -> OpaquePoint:
    return OpaquePoint(x, y, z)


@portmapping
class Point(NamedTuple):
    x: float
    y: float
    z: float


@worker.task()
def new_point(x: float, y: float, z: float) -> Point:
    return Point(x, y, z)
