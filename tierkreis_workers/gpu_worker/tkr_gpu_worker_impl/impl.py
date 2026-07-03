import logging


import torch

from tierkreis import Worker

worker = Worker("gpu_worker")
logger = logging.getLogger(__name__)


@worker.task()
def get_devices() -> str:
    """Get the available GPU devices."""
    if not torch.cuda.is_available():
        return "NaN"
    devices = [torch.cuda.get_device_name(i) for i in range(torch.cuda.device_count())]
    return ", ".join(devices)
