from importlib.metadata import version
from pathlib import Path

from fastapi import APIRouter
from fastapi.responses import FileResponse
from fastapi.staticfiles import StaticFiles
from tierkreis_visualization.routers.models import BackendInfo

PACKAGE_DIR = Path(__file__).parent.parent.absolute()
assets = StaticFiles(directory=PACKAGE_DIR / "static" / "dist" / "assets", html=True)
router = APIRouter()


@router.get("/api/info")
def get_info() -> BackendInfo:
    info = BackendInfo(version=version("tierkreis"))
    return info


@router.get("/{path:path}")
def read_root(path) -> FileResponse:
    return FileResponse(
        PACKAGE_DIR / "static" / "dist" / "index.html",
        media_type="text/html",
    )
