import json
from pathlib import Path
from fastapi import FastAPI


def generate_openapi(app: FastAPI):
    from fastapi.openapi.utils import get_openapi

    spec = get_openapi(
        title=app.title,
        version=app.version,
        openapi_version=app.openapi_version,
        description=app.description,
        routes=app.routes,
    )

    with open(Path(__file__).parent.parent / "openapi.json", "w+") as fh:
        json.dump(spec, fh)
