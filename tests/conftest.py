from pathlib import Path

import pytest
from click.testing import CliRunner


@pytest.fixture()
def runner(tmp_path: Path) -> CliRunner:
    (tmp_path / ".git").mkdir()
    runner = CliRunner()
    with runner.isolated_filesystem(temp_dir=tmp_path):
        yield runner
