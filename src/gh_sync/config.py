from __future__ import annotations

import json
from pathlib import Path
from typing import Dict, Optional

from pydantic import BaseModel, Field, validator

CONFIG_FILE = ".gh-sync.json"


class Mapping(BaseModel):
    subdir: str = Field(..., description="対象サブディレクトリ（prefix）")
    remote: str = Field(..., description="git remote 名")
    url: str = Field(..., description="リモート URL")
    branch: str = Field(..., description="追跡ブランチ")

    @validator("subdir")
    def _no_trailing_slash(cls, v: str) -> str:  # noqa: D401
        return v.rstrip("/")


class Config(BaseModel):
    mappings: Dict[str, Mapping] = Field(default_factory=dict)

    @classmethod
    def load(cls, repo_root: Path) -> "Config":
        fn = repo_root / CONFIG_FILE
        if fn.exists():
            return cls.parse_raw(fn.read_text())
        return cls()

    def save(self, repo_root: Path) -> None:
        fn = repo_root / CONFIG_FILE
        fn.write_text(self.model_dump_json(indent=2) + "\n")
