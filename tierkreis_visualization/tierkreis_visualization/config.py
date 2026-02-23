from pathlib import Path

from pydantic_settings import BaseSettings


class Settings(BaseSettings):
    tierkreis_path: Path = Path.home() / ".tierkreis" / "checkpoints"
    graph_specifier: str | None = None


CONFIG = Settings()
CONFIG.tierkreis_path.mkdir(parents=True, exist_ok=True)
TKR_GRAPH_SPECIFIER_KEY = "TIERKREIS_VIS_GRAPH_SPECIFIER"
