from pathlib import Path

from fastapi import APIRouter
from fastapi.responses import FileResponse
from fastapi.staticfiles import StaticFiles

PACKAGE_DIR = Path(__file__).parent.parent.absolute()
assets = StaticFiles(directory=PACKAGE_DIR / "static" / "dist" / "assets", html=True)
router = APIRouter()


@router.get("/{path:path}")
def read_root(path):
    return FileResponse(
        PACKAGE_DIR / "static" / "dist" / "index.html", media_type="text/html",
    )
