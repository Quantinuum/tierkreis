import io
import os
import pickle
from sys import argv
from typing import Annotated, NamedTuple
import numpy as np

from tierkreis.controller.data.core import Deserializer, Serializer
from tierkreis.worker.worker import Worker

worker = Worker("scipy_worker")


def save(a: np.ndarray) -> bytes:
    with io.BytesIO() as bs:
        np.save(bs, a)
        return bs.getvalue()


def load(bs: bytes) -> np.ndarray:
    with io.BytesIO() as bi:
        bi.write(bs)
        bi.seek(0)
        return np.load(bi, encoding="bytes")


SER_METHOD = os.environ.get("SER_METHOD")
if SER_METHOD == "dumps":
    ser = Serializer(np.ndarray.dumps)
    deser = Deserializer(pickle.loads)
elif SER_METHOD == "tolist":
    ser = Serializer(np.ndarray.tolist, "json")
    deser = Deserializer(np.array, "json")
elif SER_METHOD == "save":
    ser = Serializer(save)
    deser = Deserializer(load)
else:
    ser = None
    deser = None

NDArray = Annotated[np.ndarray, ser, deser]


class PointedArray(NamedTuple):
    a: NDArray
    p: int


@worker.task()
def add_point(a: NDArray, p: int) -> PointedArray:
    return PointedArray(a, p)


@worker.task()
def eval_point(pa: PointedArray) -> float:
    return pa.a.item(pa.p)


@worker.task()
def linspace(start: float, stop: float, num: int = 50) -> NDArray:
    return np.linspace(start, stop, num=num)


@worker.task()
def transpose(a: NDArray) -> NDArray:
    return a.transpose()


@worker.task()
def reshape(a: NDArray, shape: int | list[int]) -> NDArray:
    return np.reshape(a, shape)


if __name__ == "__main__":
    worker.app(argv)
