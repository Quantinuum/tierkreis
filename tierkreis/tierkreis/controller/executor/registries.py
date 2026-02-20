from pathlib import Path

from tierkreis.exceptions import TierkreisError


def find_registry_for_worker(
    worker_name: str,
    registry_paths: Path | list[Path],
) -> Path:
    """Return the first registry path containing a worker named `worker_name`.

    Assumes that the worker is a directory.
    """
    if isinstance(registry_paths, Path):
        registry_paths = [registry_paths]

    worker_path: Path | None = None
    for registry in registry_paths:
        subdirs = [x.stem for x in registry.iterdir() if x.is_dir()]
        if worker_name in subdirs:
            return registry
    if worker_path is None:
        msg = f"{worker_name} not in registries {registry_paths}"
        raise TierkreisError(msg)
    return None
