from pathlib import Path

from pydantic_settings import BaseSettings


class Settings(BaseSettings):
    tierkreis_path: Path = Path.home() / ".tierkreis" / "checkpoints"
    graph_specifier: str | None = None


CONFIG = Settings()
CONFIG.tierkreis_path.mkdir(parents=True, exist_ok=True)
